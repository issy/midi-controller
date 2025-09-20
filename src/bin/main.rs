#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex};
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_hal::gpio::{Input, InputConfig};

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    println!("An error has occurred! Going into panic state... {}", panic_info.message());
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static CHANNEL: Channel<CriticalSectionRawMutex, u8, 16> = Channel::new();

#[embassy_executor::task(pool_size = 6)]
async fn button_task(mut button: Input<'static>, id: u8) {
    loop {
        button.wait_for_rising_edge().await;
        println!("Button pressed");
        let _ = CHANNEL.send(id).await;
        Timer::after(Duration::from_millis(50)).await;

        button.wait_for_falling_edge().await;
        println!("Button released");
        let _ = CHANNEL.send(id).await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn logger_task() {
    loop {
        let id = CHANNEL.receive().await;
        println!("Button {} pressed/released", id);
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timer0.timer0);

    // Initialize buttons
    spawner.spawn(button_task(Input::new(peripherals.GPIO16, InputConfig::default()), 1)).unwrap();
    spawner.spawn(button_task(Input::new(peripherals.GPIO17, InputConfig::default()), 2)).unwrap();
    spawner.spawn(button_task(Input::new(peripherals.GPIO5, InputConfig::default()), 3)).unwrap();
    spawner.spawn(button_task(Input::new(peripherals.GPIO18, InputConfig::default()), 4)).unwrap();
    spawner.spawn(button_task(Input::new(peripherals.GPIO19, InputConfig::default()), 5)).unwrap();
    spawner.spawn(button_task(Input::new(peripherals.GPIO21, InputConfig::default()), 6)).unwrap();

    spawner.spawn(logger_task()).unwrap();

    println!("Started.");

    // loop {
    //     Timer::after(Duration::from_secs(1)).await;
    //     println!("Main loop tick");
    // }
}
