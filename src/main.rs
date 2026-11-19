//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use bat_device::sensors::tmf8821::config::SPAD;
use bat_device::sensors::tmf8821::device::TMF8821;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::i2c::{self, Async, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, USB};
use embassy_rp::{bind_interrupts, usb};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Timer};
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
    spawner.spawn(logger_task(p.USB).unwrap());
    let i2c: I2c<'_, I2C0, Async> =
        I2c::new_async(p.I2C0, p.PIN_1, p.PIN_0, Irqs, i2c::Config::default());
    for i in 0..3{
        Timer::after_secs(1).await;
        log::info!("{}", i);
    }
    static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
    spawner.spawn(i2c_task_a(i2c_bus).unwrap());
}

#[embassy_executor::task]
async fn i2c_task_a(i2c_bus: &'static I2cBus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let sensor = TMF8821::new(i2c_dev, 65, embassy_time::Delay).await;
    if sensor.is_err(){
        log::info!("Failed to setup sensor");
        loop {
            Timer::after_secs(1).await;
        }
    }
    log::info!("Sensor setup!!!");
    Timer::after_secs(1).await;
    let mut sensor = sensor.expect("msg");
    sensor.config.set_spad_map(SPAD::Map4x4Normal);
    let r = sensor.write_configuration().await;
    if r.is_err(){
        log::info!("Failed to write config");
        loop {
            Timer::after_secs(1).await;
        }
    }

    log::info!("Spad setup!!!");
    Timer::after_secs(1).await;
    let m = sensor.start_measurements().await;
    if m.is_err(){
        log::info!("Failed to start");
        loop {
            Timer::after_secs(1).await;
        }
    }

    log::info!("measurement setup!!!");
    Timer::after_secs(1).await;
    log::info!("TMF8821 Startup Complete!");
    loop {
        let res = sensor.wait_for_measurement().await;
        if res.is_err(){
            log::info!("Failed to get sensor measurement!!!")
        }
        else{
            let res = res.unwrap();
            let sd = res.to_sensor_data(SPAD::Map4x4Normal);
        
            log::info!("{sd:?}\r\n");
        }
        
    }
}
