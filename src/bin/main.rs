#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use esp_hal::{clock::CpuClock, delay::Delay};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pin};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    println!("An error has occurred! Going into panic state...");
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

struct Button<'a> {
    input_pin: Input<'a>,
    is_pressed: bool,
}

impl<'a> Button<'a> {
    pub fn new(input: Input<'a>) -> Self {
        Button {
            input_pin: input,
            is_pressed: false
        }
    }

    pub fn check(&mut self) -> bool {
        let currently_pressed = self.input_pin.is_high();
        if self.is_pressed == currently_pressed {
            return false;
        }
        self.is_pressed = currently_pressed;
        if currently_pressed {
            return false;
        } else {
            return true;
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    let mut button = Button::new(Input::new(peripherals.GPIO5.degrade(), InputConfig::default())); // e.g. GPIO5 as button
    let mut led = Output::new(peripherals.GPIO25.degrade(), Level::Low, OutputConfig::default()); // e.g. GPIO25 as LED

    let delay = Delay::new();

    let _spawner = spawner;

    println!("Started!");

    loop {
        if button.check() {
            led.toggle();
        }
        delay.delay_millis(50);
    }
}
