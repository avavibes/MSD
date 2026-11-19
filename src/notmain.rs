#![no_std]
#![no_main]
use core::cell::RefCell;
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::I2C0;
use embassy_time::{Delay, Timer};
// For USB
use embassy_rp::{peripherals::USB, usb};
use embedded_hal::i2c::SevenBitAddress;
use embedded_hal_bus::i2c::RefCellDevice;
use msdi2c_scan::tmf8821::device::TMF8821;
use msdi2c_scan::qmp6988::Qmp6988;
// use embedded_hal_bus::i2c;
//Panic Handler
use panic_probe as _;
embassy_rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});
#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = hal::block::ImageDef::secure_exe();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    // spawne
    spawner.spawn(logger_task(p.USB).expect("Failed to spawn"));
    let sda = p.PIN_0;
    let scl = p.PIN_1;
    // let mut i2c: embassy_rp::i2c::I2c<'static, I2C0, Async> =
    //     i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, Config::default());
    let mut i2c = I2c::new_async(
        p.I2C0,
        scl,
        sda,
        Irqs,
        I2cConfig::default(),
    );

    let i2c_ref_cell = RefCell::new(i2c);

    // let a = 

    Timer::after_millis(1000).await;
    // info!("set up i2c");
    let addr: SevenBitAddress = 65;
    // let i2c_address = SevenBitAddress(addr);
    log::info!("I2c Setup");
    // let sen = Qmp6988::new(RefCellDevice::new(&i2c_ref_cell), addr, embassy_time::Delay {});
    let tmf = TMF8821::new(RefCellDevice::new(&i2c_ref_cell), addr, embassy_time::Delay {}).unwrap();
    loop {
        Timer::after_millis(100).await;
        log::info!("I2c Setup");
        for i in 0..=127u8 {
            let mut readbuf: [u8; 1] = [0; 1];
            // i2c.
            let result = i2c.read_async(i, &mut readbuf).await;
            if let Ok(_) = result {
                log::info!("I2c response during scan at address {}", { i });
                // Do whatever work you want to do with found devices
                // writeln!(uart, "Device found at address{:?}", i).unwrap();
            }
        }
    }
}

// Program metadata for `picotool info`.
// This isn't needed, but it's recommended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"msdi2c-scan"),
    embassy_rp::binary_info::rp_program_description!(c"your program description"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

// End of file
