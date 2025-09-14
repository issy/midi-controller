#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use alloc::boxed::Box;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_hal::gpio::{AnyPin, Input, InputConfig, Pin};

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    println!("An error has occurred! Going into panic state... {}", panic_info.message());
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

struct App {
    button_ch: &'static Channel<NoopRawMutex, u8, 4>,
}

impl App {
    fn spawn_tasks(&'static self, spawner: embassy_executor::Spawner, button1: Input<'static>, button2: Input<'static>) {
        spawner.spawn(button_task(button1, 0, self.button_ch)).unwrap();
        spawner.spawn(button_task(button2, 1, self.button_ch)).unwrap();
        spawner.spawn(logger_task(self.button_ch)).unwrap();
    }
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>, id: u8, ch: &'static Channel<NoopRawMutex, u8, 4>) {
    loop {
        button.wait_for_falling_edge().await;
        let _ = ch.send(id).await;
        Timer::after(Duration::from_millis(50)).await;

        button.wait_for_rising_edge().await;
        let _ = ch.send(id).await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn logger_task(ch: &'static Channel<NoopRawMutex, u8, 4>) {
    loop {
        let id = ch.receive().await;
        println!("Button {} pressed/released", id);
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timer0.timer0);

    let _spawner = spawner;

    // Initialize buttons
    let button1: Input = Input::new(peripherals.GPIO5.degrade(), InputConfig::default());
    let button2: Input = Input::new(peripherals.GPIO18.degrade(), InputConfig::default());

    // Create app and spawn tasks
    let button_ch = Channel::<NoopRawMutex, u8, 4>::new();
    let app: App = App { button_ch: &button_ch };
    app.spawn_tasks(spawner, button1, button2);

    loop {
        Timer::after(Duration::from_secs(1)).await;
        println!("Main loop tick");
    }
}
