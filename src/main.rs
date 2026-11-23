//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use core::mem;
use core::panic::PanicInfo;

use bat_device::config::{
    DEGREE_SHIFT, ENABLE_HCSR04, ENABLE_I2S, ENABLE_START_WAIT, ENABLE_TEST_FPSIN, ENABLE_TMF8821,
    ENABLE_USB_LOGGING, I2C0_FREQ, USB_LOG_SIZE,
};
use bat_device::core::utils::panic_wait;
use bat_device::sensors::core::utils::{fixed_to_float, fpcos, fpsin};
use bat_device::sensors::tmf8821::config::SPAD;
use bat_device::sensors::tmf8821::device::TMF8821;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Async, I2c};
use embassy_rp::peripherals::{DMA_CH0, I2C0, PIO0, USB};
use embassy_rp::pio::{self, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_rp::{bind_interrupts, dma, usb};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Delay, Duration, Instant, Timer};
use hcsr04_async::{DistanceUnit, Hcsr04, Now, TemperatureUnit};
use static_cell::StaticCell;

type I2cBus = Mutex<NoopRawMutex, I2c<'static, I2C0, Async>>;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
        log::error!("{info}");
    }
}
bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(USB_LOG_SIZE, log::LevelFilter::Info, driver);
}

#[embassy_executor::main()]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    if ENABLE_USB_LOGGING {
        spawner.spawn(logger_task(p.USB).unwrap());
    }

    if ENABLE_START_WAIT {
        for i in 0..3 {
            Timer::after_secs(1).await;
            log::info!("Startup wait -- {}", i);
        }
    }
    if ENABLE_TEST_FPSIN {
        for val in [0.0, 90.0, 180.1, 270.0, 360.0, 3.5, 8.0, 20.0, 30.0, 45.0] {
            // let val = 3.5;
            let test = (val * ((1 << DEGREE_SHIFT) as f64)) as i32;
            let test_as_f = fixed_to_float(test);
            let fastsin = fpsin(test);
            let as_f = fixed_to_float(fastsin);
            let fastcos = fpcos(test);
            let c_as_f = fixed_to_float(fastcos);
            log::info!("{val} - {test} - {test_as_f} - {fastsin} -- {as_f} - {c_as_f}");
        }
    }
    if ENABLE_TMF8821 {
        let tmf_enable_pin = Output::new(p.PIN_3, Level::Low);
        let mut i2c_config = i2c::Config::default();
        i2c_config.frequency = I2C0_FREQ;
        let i2c: I2c<'_, I2C0, Async> = I2c::new_async(p.I2C0, p.PIN_1, p.PIN_0, Irqs, i2c_config);

        static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
        let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
        spawner.spawn(i2c_task_a(i2c_bus, tmf_enable_pin).unwrap());
    }
    if ENABLE_I2S {
        // PIO I2s Audio
        const SAMPLE_RATE: u32 = 44100;
        const BIT_DEPTH: u32 = 16;
        let Pio {
            mut common, sm0, ..
        } = Pio::new(p.PIO0, Irqs);
        let fade_input = Input::new(p.PIN_10, Pull::Up);
        let bit_clock_pin = p.PIN_18;
        let left_right_clock_pin = p.PIN_19;
        let data_pin = p.PIN_20;

        let program = PioI2sOutProgram::new(&mut common);
        let i2s: PioI2sOut<'_, PIO0, 0> = PioI2sOut::new(
            &mut common,
            sm0,
            p.DMA_CH0,
            Irqs,
            data_pin,
            bit_clock_pin,
            left_right_clock_pin,
            SAMPLE_RATE,
            BIT_DEPTH,
            &program,
        );
        spawner.spawn(i2s_task_a(i2s, fade_input).unwrap());
    }
    if ENABLE_HCSR04 {
        let trigger_pin = Output::new(p.PIN_21, Level::Low);
        let echo_pin = Input::new(p.PIN_22, Pull::Down);
        spawner.spawn(hcsr04_task(trigger_pin, echo_pin).unwrap());
    }
}
#[embassy_executor::task]
async fn hcsr04_task(trigger_pin: Output<'static>, echo_pin: Input<'static>) {
    let config = hcsr04_async::Config {
        distance_unit: DistanceUnit::Centimeters,
        temperature_unit: TemperatureUnit::Celsius,
    };
    struct EmbassyClock;

    impl Now for EmbassyClock {
        fn now_micros(&self) -> u64 {
            Instant::now().as_micros()
        }
    }
    let clock = EmbassyClock;
    let delay = Delay;

    let mut sensor = Hcsr04::new(trigger_pin, echo_pin, config, clock, delay);

    // The temperature of the environment, if known, can be used to adjust the speed of sound.
    // If unknown, an average estimate must be used.
    let temperature = 24.0;

    loop {
        let distance = sensor.measure(temperature).await;
        match distance {
            Ok(distance) => {
                log::info!("HCSR04_distane: {} cm", distance);
            }
            Err(e) => {
                log::info!("HCSR04 ERROR: {:?}", e);
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}

#[embassy_executor::task]
async fn i2s_task_a(mut i2s: PioI2sOut<'static, PIO0, 0>, fade: Input<'static>) {
    i2s.start();

    const BUFFER_SIZE: usize = 960;
    static DMA_BUFFER: StaticCell<[u32; BUFFER_SIZE * 2]> = StaticCell::new();
    let dma_buffer = DMA_BUFFER.init_with(|| [0u32; BUFFER_SIZE * 2]);
    let (mut back_buffer, mut front_buffer) = dma_buffer.split_at_mut(BUFFER_SIZE);

    let mut fade_value: i32 = 0;
    let mut phase: i32 = 0;
    loop {
        // trigger transfer of front buffer data to the pio fifo
        // but don't await the returned future, yet
        let dma_future = i2s.write(front_buffer);

        // fade in audio when GPIO 0 pin is shorted to GND
        let fade_target = if fade.is_low() { i32::MAX } else { 0 };

        // fill back buffer with fresh audio samples before awaiting the dma future
        for s in back_buffer.iter_mut() {
            // exponential approach of fade_value => fade_target
            fade_value += (fade_target - fade_value) >> 14;
            // generate triangle wave with amplitude and frequency based on fade value
            phase = (phase + (fade_value >> 22)) & 0xffff;
            let triangle_sample = (phase as i16 as i32).abs() - 16384;
            let sample = (triangle_sample * (fade_value >> 15)) >> 16;
            // duplicate mono sample into lower and upper half of dma word
            *s = (sample as u16 as u32) * 0x10001;
        }

        // now await the dma future. once the dma finishes, the next buffer needs to be queued
        // within DMA_DEPTH / SAMPLE_RATE = 8 / 48000 seconds = 166us
        dma_future.await;
        mem::swap(&mut back_buffer, &mut front_buffer);
    }
}

#[embassy_executor::task]
async fn i2c_task_a(i2c_bus: &'static I2cBus, pin: Output<'static>) {
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
    let res = sensor.single_measurement().await;
    if let Ok(res) = res {
        log::info!("{res}");
    } else {
        log::info!("Failed to get sensor measurement!!!");
        Timer::after_millis(1000).await;
    }
    let _ = sensor.standby().await;
    Timer::after_millis(1000).await;
    // let res = sensor.single_measurement().await;
    // if let Ok(res) = res {
    //     log::info!("{res}");
    // } else {
    //     log::info!("Failed to get sensor measurement!!!");
    //     Timer::after_millis(1000).await;
    // }
    let now = Instant::now();
    let _ = sensor.wake().await;
    log::info!("Wake took {} ms", now.elapsed().as_millis());
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
        if let Ok(res) = res {
            log::info!("{res}");
        } else {
            log::info!("Failed to get sensor measurement!!!");
            Timer::after_millis(1000).await;
        }
        // panic_wait("exited").await;
    }
}
