//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// use defmt::*;
// use defmt_rtt as _;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{self, Async, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, USB};
use embassy_rp::{bind_interrupts, usb};
// use embassy_rp::{bind_interrupts, dma, mode};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use msdi2c_scan::tmf8821::config::SPAD;
use msdi2c_scan::tmf8821::device::{self, TMF8821};
// use panic_probe as _;
use static_cell::StaticCell;

// type SpiBus = Mutex<NoopRawMutex, Spi<'static, mode::Async>>;
type I2cBus = Mutex<NoopRawMutex, I2c<'static, I2C0, Async>>;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
        log::error!("{}", info);
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
    let p = embassy_rp::init(Default::default());
    log::info!("Here we go!");
    spawner.spawn(logger_task(p.USB).unwrap());
    // Shared I2C bus
    // I2c::
    let i2c: I2c<'_, I2C0, Async> =
        I2c::new_async(p.I2C0, p.PIN_1, p.PIN_0, Irqs, i2c::Config::default());
    static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
    // for i in 0..5{

    //     Timer::after_secs(1).await;

    //     log::info!("{i}");
    // }
    spawner.spawn(i2c_task_a(i2c_bus).unwrap());
    // spawner.spawn(i2c_task_b(i2c_bus).unwrap());

    // Shared SPI bus
    // let spi_cfg = spi::Config::default();
    // let spi = Spi::new(
    //     p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, p.DMA_CH0, p.DMA_CH1, Irqs, spi_cfg,
    // )
    // .unwrap();
    // static SPI_BUS: StaticCell<SpiBus> = StaticCell::new();
    // let spi_bus = SPI_BUS.init(Mutex::new(spi));

    // // Chip select pins for the SPI devices
    // let cs_a = Output::new(p.PIN_0, Level::High);
    // let cs_b = Output::new(p.PIN_1, Level::High);

    // spawner.spawn(spi_task_a(spi_bus, cs_a).unwrap());
    // spawner.spawn(spi_task_b(spi_bus, cs_b).unwrap());
}
// fn
#[embassy_executor::task]
async fn i2c_task_a(i2c_bus: &'static I2cBus) {
    log::info!("i2c task A");
    let i2c_dev = I2cDevice::new(i2c_bus);

    log::info!("i2c task b");
    let sensor = TMF8821::new(i2c_dev, 65, embassy_time::Delay).await;

    log::info!("i2c task c");
    // Timer::after_secs(1).await;

    // let _sensor = DummyI2cDeviceDriver::new(i2c_dev, 0xC0);
    let mut sensor = sensor.expect("msg");
    sensor.config.set_spad_map(SPAD::MAP_4x4_normal);
    sensor.write_configuration().await;
    sensor.start_measurements().await.expect("msg");
    loop {
        // log::info!("i2c task A");
        let res = sensor.wait_for_measurement().await.unwrap();
        log::info!("{}\r\n", res);

        // Timer::after_millis(200).await;
    }
}

// #[embassy_executor::task]
// async fn i2c_task_b(i2c_bus: &'static I2cBus) {
//     let i2c_dev = I2cDevice::new(i2c_bus);
//     let _sensor = DummyI2cDeviceDriver::new(i2c_dev, 0xDE);
//     loop {
//         log::info!("i2c task B");
//         Timer::after_secs(1).await;
//     }
// }

// #[embassy_executor::task]
// async fn spi_task_a(spi_bus: &'static SpiBus, cs: Output<'static>) {
//     let spi_dev = SpiDevice::new(spi_bus, cs);
//     let _sensor = DummySpiDeviceDriver::new(spi_dev);
//     loop {
//         log::info!("spi task A");
//         Timer::after_secs(1).await;
//     }
// }

// #[embassy_executor::task]
// async fn spi_task_b(spi_bus: &'static SpiBus, cs: Output<'static>) {
//     let spi_dev = SpiDevice::new(spi_bus, cs);
//     let _sensor = DummySpiDeviceDriver::new(spi_dev);
//     loop {
//         log::info!("spi task B");
//         Timer::after_secs(1).await;
//     }
// }

// Dummy I2C device driver, using `embedded-hal-async`
// struct DummyI2cDeviceDriver<I2C: embedded_hal_async::i2c::I2c> {
//     _i2c: I2C,
// }

// impl<I2C: embedded_hal_async::i2c::I2c> DummyI2cDeviceDriver<I2C> {
//     fn new(i2c_dev: I2C, _address: u8) -> Self {
//         Self { _i2c: i2c_dev }
//     }
// }

// // Dummy SPI device driver, using `embedded-hal-async`
// struct DummySpiDeviceDriver<SPI: embedded_hal_async::spi::SpiDevice> {
//     _spi: SPI,
// }

// impl<SPI: embedded_hal_async::spi::SpiDevice> DummySpiDeviceDriver<SPI> {
//     fn new(spi_dev: SPI) -> Self {
//         Self { _spi: spi_dev }
//     }
// }
