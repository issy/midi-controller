#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    println!(
        "An error has occurred! Going into panic state... {}",
        panic_info.message()
    );
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const BUTTONS_AMOUNT: usize = 6;

// TODO: Include MIDI message types in here
enum ChannelEvent {
    ActivateScene { button_id: u8 },
    MomentaryPressed { button_id: u8 },
    MomentaryReleased { button_id: u8 },
}

enum ButtonMode {
    Scene { id: u8 },
    Momentary { id: u8 },
}

static CHANNEL: Channel<CriticalSectionRawMutex, ChannelEvent, 16> = Channel::new();

#[embassy_executor::task(pool_size = BUTTONS_AMOUNT)]
async fn button_task(mut button: Input<'static>, mode: ButtonMode) {
    match mode {
        ButtonMode::Momentary { id } => loop {
            button.wait_for_rising_edge().await;
            // Button pressed
            println!("Button pressed: {}", id);
            CHANNEL
                .send(ChannelEvent::MomentaryPressed { button_id: id })
                .await;
            Timer::after(Duration::from_millis(50)).await;

            button.wait_for_falling_edge().await;
            // Button released
            println!("Button released: {}", id);
            CHANNEL
                .send(ChannelEvent::MomentaryReleased { button_id: id })
                .await;
            Timer::after(Duration::from_millis(50)).await;
        },
        ButtonMode::Scene { id } => loop {
            button.wait_for_rising_edge().await;
            // Button pressed
            Timer::after(Duration::from_millis(50)).await;

            button.wait_for_falling_edge().await;
            // Button released
            CHANNEL
                .send(ChannelEvent::ActivateScene { button_id: id })
                .await;
            Timer::after(Duration::from_millis(50)).await;
        },
    }
}

#[embassy_executor::task]
async fn led_watchdog(mut leds: [(Output<'static>, ButtonMode); 6]) {
    loop {
        let event = CHANNEL.receive().await;
        match event {
            ChannelEvent::ActivateScene { button_id } => {
                println!("Received ActivateScene {}", button_id);
                if button_id >= leds.len() as u8 {
                    return;
                }
                leds.iter_mut().for_each(|(led, led_mode)| match led_mode {
                    ButtonMode::Scene { id } => {
                        if *id == button_id {
                            led.set_high();
                        } else {
                            led.set_low();
                        }
                    }
                    _ => (),
                })
            },
            ChannelEvent::MomentaryPressed { button_id } => {
                match leds.iter_mut().find(|(_, led_mode)| match led_mode {
                    ButtonMode::Momentary { id } => *id == button_id,
                    _ => false
                }) {
                    Some((led, _)) => led.set_high(),
                    None => ()
                }
            }
            ChannelEvent::MomentaryReleased { button_id } => {
                match leds.iter_mut().find(|(_, led_mode)| match led_mode {
                    ButtonMode::Momentary { id } => *id == button_id,
                    _ => false
                }) {
                    Some((led, _)) => led.set_low(),
                    None => ()
                }
            },
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timer0.timer0);

    let leds = [
        (
            Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default()),
            ButtonMode::Momentary { id: 0 },
        ),
        (
            Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default()),
            ButtonMode::Momentary { id: 1 },
        ),
        (
            Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default()),
            ButtonMode::Scene { id: 2 },
        ),
        (
            Output::new(peripherals.GPIO25, Level::Low, OutputConfig::default()),
            ButtonMode::Scene { id: 3 },
        ),
        (
            Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default()),
            ButtonMode::Scene { id: 4 },
        ),
        (
            Output::new(peripherals.GPIO32, Level::Low, OutputConfig::default()),
            ButtonMode::Scene { id: 5 },
        ),
    ];
    spawner.spawn(led_watchdog(leds)).unwrap();

    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO16, InputConfig::default()),
            ButtonMode::Momentary { id: 0 },
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO17, InputConfig::default()),
            ButtonMode::Momentary { id: 1 },
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO5, InputConfig::default()),
            ButtonMode::Scene { id: 2 },
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO18, InputConfig::default()),
            ButtonMode::Scene { id: 3 },
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO19, InputConfig::default()),
            ButtonMode::Scene { id: 4 },
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO21, InputConfig::default()),
            ButtonMode::Scene { id: 5 },
        ))
        .unwrap();

    println!("Started.");
}
