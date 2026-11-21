//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use bat_device::core::utils::panic_wait;
use bat_device::sensors::tmf8821::config::SPAD;
use bat_device::sensors::tmf8821::device::TMF8821;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{self, Async, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, USB};
use embassy_rp::{bind_interrupts, usb};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use static_cell::StaticCell;

type I2cBus = Mutex<NoopRawMutex, I2c<'static, I2C0, Async>>;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
        log::error!("{info}");
    }
}
bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});
#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main()]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let tmf_enable_pin = Output::new(p.PIN_3, Level::Low);
    spawner.spawn(logger_task(p.USB).unwrap());
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 400000;
    let i2c: I2c<'_, I2C0, Async> =
        I2c::new_async(p.I2C0, p.PIN_1, p.PIN_0, Irqs, i2c_config);
    for i in 0..3 {
        Timer::after_secs(1).await;
        log::info!("Startup wait -- {}", i);
    }
    static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
    spawner.spawn(i2c_task_a(i2c_bus, tmf_enable_pin).unwrap());
}

#[embassy_executor::task]
async fn i2c_task_a(i2c_bus: &'static I2cBus,pin: Output<'static> ) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    // let sensors = []
    let mut sensor = TMF8821::new(i2c_dev, 66, embassy_time::Delay, pin).await;
    
    let res = sensor.init().await;
    
    if res.is_err() {
        panic_wait("Failed to setup sensor").await;
    }
    
    // sensor.config.set_i2c_addr(66);
    // sensor.config
    log::info!("Sensor setup!!!");
    sensor.config.set_spad_map(SPAD::Map4x4Normal);

    // let mut sensor = sensor;
    let r = sensor.write_configuration().await;
    if r.is_err() {
        panic_wait("Failed to write config to sensor").await;
    }
    sensor.config.set_spad_map(SPAD::Map3x3Checkerboard);
    let r = sensor.write_configuration().await;
    if r.is_err() {
        panic_wait("Failed to write config to sensor").await;
    }
    // let r = sensor.set_addr().await;
    // if r.is_err() {
    //     panic_wait("Failed to set addr").await;
    // }
    // let res = sensor.set_addr().await;
    // if res.is_err() {
    //     panic_wait("Failed to change address").await;
    // }
    // let m = sensor.start_measurements().await;
    // if m.is_err() {
    //     log::info!("{}", sensor.get_address());
    //     panic_wait("Failed to start tmf8821 measurements").await;
    // }
    // log::info!("TMF8821 Startup Complete!");
    // let res = sensor.wait_for_measurement().await;
    //     if let Ok(res) = res{
    //         log::info!("{res}");
    //     }
    //     else {
    //         log::info!("Failed to get sensor measurement!!!");
    //         Timer::after_millis(1000).await;
    //     }
    // log::info!("SETTING SPAD DIFF");
    loop {
        let res = sensor.single_measurement().await;
        if let Ok(res) = res{
            log::info!("{res}");
        }
        else {
            log::info!("Failed to get sensor measurement!!!");
            Timer::after_millis(1000).await;
        }
        // panic_wait("exited").await;
    }
}
