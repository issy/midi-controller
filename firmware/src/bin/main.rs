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

// Include MIDI message field
enum ChannelEvent {
    ActivateScene { button_id: u8 },
    MomentaryPressed { button_id: u8 },
    MomentaryReleased { button_id: u8 },
}

// TODO: Include MIDI message types in here
enum ButtonConfiguration {
    MomentaryButton(),
    SceneButton(u8),
}

static CHANNEL: Channel<CriticalSectionRawMutex, ChannelEvent, 16> = Channel::new();

#[embassy_executor::task(pool_size = BUTTONS_AMOUNT)]
async fn button_task(mut button: Input<'static>, button_id: u8) {
    loop {
        button.wait_for_rising_edge().await;
        // Button pressed
        Timer::after(Duration::from_millis(50)).await;

        button.wait_for_falling_edge().await;
        // Button released
        let _ = CHANNEL
            .send(ChannelEvent::ActivateScene { button_id })
            .await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task(pool_size = BUTTONS_AMOUNT)]
async fn button_task_momentary(mut button: Input<'static>, button_id: u8) {
    loop {
        button.wait_for_rising_edge().await;
        println!("Button pressed");
        let _ = CHANNEL
            .send(ChannelEvent::MomentaryPressed { button_id })
            .await;
        Timer::after(Duration::from_millis(50)).await;

        button.wait_for_falling_edge().await;
        println!("Button released");
        let _ = CHANNEL
            .send(ChannelEvent::MomentaryReleased { button_id })
            .await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn led_watchdog(mut leds: [Output<'static>; 6]) {
    loop {
        let event = CHANNEL.receive().await;
        match event {
            ChannelEvent::ActivateScene { button_id } => {
                if button_id < leds.len() as u8 {
                    leds.iter_mut()
                        .enumerate()
                        .for_each(|(i, led)| match i as u8 == button_id {
                            true => {
                                led.set_high();
                            }
                            false => {
                                led.set_low();
                            }
                        })
                }
            }
            _ => {}
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
        Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO25, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO32, Level::Low, OutputConfig::default()),
    ];
    spawner.spawn(led_watchdog(leds)).unwrap();

    // Initialize buttons
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO16, InputConfig::default()),
            0,
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO17, InputConfig::default()),
            1,
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO5, InputConfig::default()),
            2,
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO18, InputConfig::default()),
            3,
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO19, InputConfig::default()),
            4,
        ))
        .unwrap();
    spawner
        .spawn(button_task(
            Input::new(peripherals.GPIO21, InputConfig::default()),
            5,
        ))
        .unwrap();

    println!("Started.");
}
