//! This example shows how to share (async) I2C and SPI buses between multiple devices.

#![no_std]
#![no_main]

use core::mem;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicI32, Ordering};

use bat_device::config::{
    ENABLE_HCSR04, ENABLE_I2S, ENABLE_LED_BLINK, ENABLE_START_WAIT, ENABLE_TEMP_SENSOR_HCSR04,
    ENABLE_TEST_FPSIN, ENABLE_TMF8821, ENABLE_USB_LOGGING, ENABLE_WATCHDOG_PANIC, FIXED_POINT_TEST,
    I2C0_FREQ, LOOP_SPAD_MAPS, USB_LOG_SIZE,
};
use bat_device::core::math::FixedPoint;
use bat_device::core::utils::panic_wait;
use bat_device::detection::{Area3D, DetectionArea};
use bat_device::sensors::core::utils::fpsin;
use bat_device::sensors::core::{Sensor, SensorType};
use bat_device::sensors::tmf8821::config::SPAD;
use bat_device::sensors::tmf8821::device::TMF8821;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config as AdcConfig};
use embassy_rp::config::Config;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Async, I2c};
use embassy_rp::peripherals::{ADC_TEMP_SENSOR, DMA_CH0, I2C0, PIO0, USB};
use embassy_rp::pio::{self, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_rp::{adc, bind_interrupts, dma, usb};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_task_watchdog::embassy_rp::{TaskWatchdog, WatchdogRunner};
use embassy_task_watchdog::{WatchdogConfig, create_watchdog};
use embassy_time::{Delay, Duration, Instant, Timer};
use hcsr04_async::{DistanceUnit, Hcsr04, Now, TemperatureUnit};
use static_cell::StaticCell;

type I2cBus = Mutex<NoopRawMutex, I2c<'static, I2C0, Async>>;

static SHARED_TEMP: AtomicI32 = AtomicI32::new(0);
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
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(USB_LOG_SIZE, log::LevelFilter::Info, driver);
}

#[embassy_executor::main()]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let mut adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    // let temp_val: AtomicI32 = AtomicI32::new(FixedPoint::from(22).raw());
    // let temp_value = FixedPoint::from(22);
    // static TEMP_VAL: StaticCell<Mutex<NoopRawMutex, FixedPoint>> = StaticCell::new();
    // let temp_val = TEMP_VAL.init(Mutex::new(temp_value));
    // // let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
    // if ENABLE_TEMP_SENSOR_HCSR04{
    //     spawner.spawn(temp_sensor_task(p.))
    // }
    if ENABLE_USB_LOGGING {
        spawner.spawn(logger_task(p.USB).unwrap());
    }

    if ENABLE_START_WAIT {
        for i in 0..3 {
            Timer::after_secs(1).await;
            log::info!("Startup wait -- {}", i);
        }
    }
    if FIXED_POINT_TEST {
        let a = FixedPoint::from(3.3);
        let b = FixedPoint::from(7.2);
        let c = a + b;
        let d = a * b;

        log::info!(
            "FIXED POINT add {}+{}={}",
            f64::from(a),
            f64::from(b),
            f64::from(c)
        );
        log::info!(
            "FIXED POINT multiply {}*{}={}",
            f64::from(a),
            f64::from(b),
            f64::from(d)
        );

        log::info!(
            "FIXED POINT multiply 2*{}*{}={}",
            f64::from(a),
            f64::from(b),
            f64::from(d * 2)
        );
    }
    if ENABLE_TEST_FPSIN {
        // let vals = [223.94862173466942, 202.45924966826803, 314.2891611206414, 309.4907240026413, 270.8613453291833, 146.79427504801194, 123.53744875671048, 203.0655341872093, 239.20959504611977, 269.11784967746866, 280.9130736046719, 359.86706853308937, 66.80167973992585, 165.80188280171632, 153.87507978389124, 229.64140921653845, 19.174626834008812, 150.51257971691746, 285.6036505730694, 184.62907143786055, 276.8477422837907, 270.00231547319527, 107.2071936407867, 167.21985508945448, 282.8798210833321, 342.01523939221727, 325.7123197856753, 163.44305914386445, 72.00090775204863, 41.02616936214146, 278.9193032158314, 89.48025371858888, 234.84138676860806, 108.3565605143333, 32.99238048602707, 34.24152151154325, 268.92094944710607, 283.23700927025766, 204.9928771359978, 207.61138051990054, 350.805952884948, 256.2017177461483, 228.31491557360914, 220.41926072639802, 304.8987183382827, 316.1032866317256, 20.081040000039422, 7.512444982969164, 252.00117326489362, 87.02365404606891, 22.727629296440046, 60.029077086594285, 135.2738530829712, 118.43567892981368, 278.54296584370496, 200.963133100484, 17.4498060495125, 198.0057494715746, 266.25074781194, 73.33141176373462, 38.47390378200668, 108.64577130283936, 143.55323781199579, 95.50790021461088, 0.7781755194831685, 333.15817956291454, 119.87757687472316, 51.37208745368751, 149.13894279143003, 35.58438912037978, 327.78164409475085, 260.0644331538854, 324.3010953199056, 254.41302505291986, 64.8607693164219, 317.39227335772813, 332.81522019704425, 23.538453602446488, 232.22758237455835, 317.1861220428655, 209.25379187184893, 296.9954949647851, 337.4844745477852, 71.09765615151886, 108.63755094094522, 140.04319936415584, 306.6317591450364, 44.18631229302483, 77.4059338619707, 229.3529244116261, 56.02590880306501, 199.0390605456259, 183.18129519190984, 16.454527181338143, 16.563686426651955, 18.663925665837304, 337.0936386362658, 168.6998862366933, 179.26035180204775, 28.664738014518313, 94.47255489988257, 35.42030231435922, 195.42099649958067, 195.42157683690584, 136.1702355265394, 270.3979891490368, 25.901493029073784, 29.686976004774912, 136.08635938140927, 33.0036521992306, 169.74361165501114, 36.27531364607199, 359.5388982575836, 116.47463750131563, 23.61502031820443, 104.60713331514262, 307.277926696584, 228.78634573097787, 256.3000982004087, 41.91116779746767, 116.34754286046724, 93.80327436713819, 107.44675907321843, 249.72283040858727, 31.155528759669757, 40.53813271774146, 180.02082810099054, 75.16118904175536, 269.7662671713442, 283.2470444715448, 251.904041435066, 277.9255182262909, 306.6811933402882, 299.9281682448227, 169.71732886020317, 44.41143968459799, 55.66408870630774, 105.85798479646043, 19.37073180949752, 355.8511328886569, 92.17034313324453, 144.48367001836513, 173.0168253677932, 319.518720927628, 173.0577502563313, 19.91901545321003, 321.2980447506058, 140.72255311060164, 143.71992522517596, 251.07922946448912, 69.33557781101571, 82.50849334768206, 250.05437872730116, 23.72028942258102, 76.10381265191033, 75.33335538137368, 33.275026088940855, 80.04839704349355, 25.829128784498412, 86.64517547967688, 119.90220520023821, 154.99686155783874, 87.32349098701717, 107.26083101099215, 96.06439146033686, 163.9682755645013, 202.06680616573803, 158.42256853643974, 179.8776140372664, 342.2460997530257, 77.55185679310148, 114.27240929851214, 248.1015346445377, 87.47502392336938, 1.6665193595452532, 147.91639265652125, 51.094155510960945, 181.25554478074417, 164.85656264380458, 26.29222768235495];
        // let vals_i = [
        //     234827149, 212293910, 329556071, 324524545, 284018706, 153924953, 129538403, 212929645,
        //     250829440, 282190518, 294558707, 377347971, 70046638, 173855875, 161349715, 240796470,
        //     20106053, 157823878, 299477133, 193597613, 290295898, 283117947, 112414890, 175342726,
        //     296620991, 358628971, 341534121, 171382469, 75498423, 43019056, 292468087, 93826846,
        //     246249041, 113620088, 34595018, 35904837, 281984053, 296995530, 214950611, 217696310,
        //     367846702, 268646972, 239405540, 231126346, 319709478, 331458319, 21056496, 7877369,
        //     264242382, 91250915, 23831646, 62945049, 141844915, 124188810, 292073468, 210725118,
        //     18297447, 207624076, 279184144, 76893558, 40342812, 113923348, 150526479, 100147291,
        //     815976, 349341671, 125700750, 53867537, 156383516, 37312936, 343703965, 272697323,
        //     340054345, 266771392, 68011446, 332809920, 348982052, 24681857, 243508269, 332593755,
        //     219418504, 311422348, 353878120, 74551295, 113914728, 146845937, 321526703, 46332706,
        //     81166004, 240493972, 58747423, 208707581, 192079509, 17253822, 17368284, 19570544,
        //     353468299, 176894651, 187968102, 30057156, 99061653, 37140878, 204913766, 204914375,
        //     142784840, 283532841, 27159683, 31129050, 142696890, 34606837, 177989077, 38037423,
        //     377003859, 122132509, 24762143, 109688529, 322204259, 239899871, 268750131, 43947044,
        //     121999241, 98359862, 112666092, 261853366, 32668939, 42507313, 188765519, 78812218,
        //     282870433, 297006052, 264140532, 291426028, 321578538, 314497478, 177961517, 46568769,
        //     58368027, 111000142, 20311684, 373136957, 96647609, 151502108, 181421290, 335039662,
        //     181464203, 20886601, 336905418, 147558291, 150701264, 263275654, 72703622, 86516425,
        //     262201020, 24872526, 79800631, 78992748, 34891393, 83936827, 27083804, 90854051,
        //     125726574, 162525989, 91565316, 112471133, 100730815, 171933198, 211882403, 166118103,
        //     188615349, 358871046, 81319015, 119823305, 260153314, 91724210, 1747472, 155101579,
        //     53576105, 190060214, 172864635, 27569398,
        // ];
        // let mut results: [f64; 180] = [0.0; 180];
        // let mut results2: [i32; 180] = [0; 180];
        // let now = Instant::now();
        // for _ in 0..1000 {
        for i in -180..180 {
            // let val = 3.5;
            // let test = vals_i[i];
            let test = FixedPoint::from(i);
            let test = test + FixedPoint::raw_from(1);
            // let test = FixedPoint::raw_from((i << DEGREE_SHIFT) + 5);
            // // let test = (val * ((1 << DEGREE_SHIFT) as f64)) as i32;
            // // let test_as_f = fixed_to_float(test);
            let fastsin = fpsin(test);
            // // results2[i] = fastsin;
            log::info!("{}", f64::from(fastsin));
            Timer::after_millis(1).await;
            // let as_f = fixed_to_float(fastsin);
            // let fastcos = fpcos(test);
            // let c_as_f = fixed_to_float(fastcos);
            // log::info!("{test} - {test} - {test_as_f} - {fastsin} -- {as_f} - {c_as_f}");
        }
        // }
        // let elapsed = now.elapsed().as_micros();
        // log::info!(
        //     "Fixed math took {elapsed}us to do {} operations",
        //     1000 * 180
        // );
        // let now = Instant::now();
        // for i in 0..180{
        //     let test = vals[i];
        //     results[i] = core::f64::sin((test*f64::consts::PI/180.0));
        // }
        // let elapsed = now.elapsed().as_micros();
        // log::info!("Float math took {elapsed}us");
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
        // if ENABLE_TEMP_SENSOR_HCSR04 {

        // }

        // let temp = adc.read(&mut ts).await.unwrap();
        // log::info!("Temp: {} degrees", f64::from(convert_to_celsius(temp)));
        spawner.spawn(hcsr04_task(trigger_pin, echo_pin).unwrap());
    }
    if ENABLE_LED_BLINK {
        let led = Output::new(p.PIN_25, Level::Low);
        spawner.spawn(led_flash(led).unwrap());
    }
    // let temp_channel = Channel
    let mut p26 = Channel::new_pin(p.PIN_26, Pull::None);
    let mut p27 = Channel::new_pin(p.PIN_27, Pull::None);
    let mut p28 = Channel::new_pin(p.PIN_28, Pull::None);

    // loop {
    //     let temp = adc.read(&mut ts).await.unwrap();
    //     log::info!("Temp: {} degrees", f64::from(convert_to_celsius(temp)));
    //     Timer::after_millis(1).await;
    // }

    if ENABLE_WATCHDOG_PANIC {
        let (watchdog, watchdogtask) = create_watchdog!(p.WATCHDOG, WatchdogConfig::default());
        spawner.spawn(watchdog_task(watchdogtask).unwrap());
        spawner.spawn(feed_watchdog(watchdog).unwrap());
    }
    if ENABLE_TEMP_SENSOR_HCSR04 {
        let mut ts = Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
        loop {
            let temp = adc.read(&mut ts).await.unwrap();
            Timer::after_millis(500).await;
            let val = convert_to_celsius(temp);
            SHARED_TEMP.store(val.num, Ordering::Relaxed);
        }
    }
}

#[embassy_task_watchdog::task(timeout = Duration::from_millis(1500))]
async fn feed_watchdog(watchdog: TaskWatchdog) -> ! {
    loop {
        watchdog.feed().await;
        Timer::after_millis(750).await;
    }
}
#[embassy_executor::task]
async fn watchdog_task(watchdog: WatchdogRunner) -> ! {
    watchdog.run().await
}
#[embassy_executor::task]
async fn led_flash(mut led: Output<'static>) {
    loop {
        led.set_high(); // Turn on the LED
        Timer::after_millis(200).await;

        led.set_low(); // Turn off the LED
        Timer::after_millis(200).await;
    }
}
#[embassy_executor::task]
async fn panic_after(millis: u64) {
    Timer::after_millis(millis).await;
    panic!();
}

fn convert_to_celsius(raw_temp: u16) -> FixedPoint {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    let temp = FixedPoint::from(raw_temp as i32);
    let temp = FixedPoint::from(27)
        - (temp * FixedPoint::from(3.3) / FixedPoint::from(4096) - FixedPoint::from(0.706))
            / FixedPoint::from(0.001721);
    let sign = if temp < FixedPoint::from(0) {
        FixedPoint::from(-1)
    } else {
        FixedPoint::from(1)
    };
    return temp * sign;
    // let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    // (rounded_temp_x10 as f32) / 10.0
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
    let temperature: f64 = (FixedPoint::raw_from(SHARED_TEMP.load(Ordering::Relaxed))).into();

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
    if LOOP_SPAD_MAPS {
        loop {
            for i in [
                SPAD::Map3x3Normal,
                SPAD::Map3x3Wide,
                SPAD::Map3x3MacroUpper,
                SPAD::Map3x3MacroLower,
                SPAD::Map3x3Checkerboard,
                SPAD::Map3x3InvCheckerboard,
                SPAD::Map4x4Narrow,
                SPAD::Map4x4Normal,
                SPAD::Map4x4MacroUpper,
                SPAD::Map4x4MacroLower,
                SPAD::Map3x6,
            ] {
                // let now = Instant::now();
                sensor.config.set_spad_map(i);
                let r = sensor.write_configuration().await;
                if r.is_err() {
                    panic_wait("Failed to write config to sensor").await;
                }
                // let t = now.elapsed();
                for _ in 0..100 {
                    let res = sensor.single_measurement().await;
                    if let Ok(res) = res {
                        log::info!("{res}");
                    } else {
                        log::info!("Failed to get sensor measurement!!!");
                        Timer::after_millis(100).await;
                    }
                }

                // log::info!("Time to switch mode: {}ms", t.as_millis());
            }
        }
    }
    // let res = sensor.single_measurement().await;
    // if let Ok(res) = res {
    //     log::info!("{res}");
    // } else {
    //     log::info!("Failed to get sensor measurement!!!");
    //     Timer::after_millis(1000).await;
    // }

    // let _ = sensor.standby().await;
    // Timer::after_millis(1000).await;
    // let now = Instant::now();
    // let _ = sensor.wake().await;
    // log::info!("Wake took {} ms", now.elapsed().as_millis());

    sensor.config.set_spad_map(SPAD::Map4x4Narrow);

    // let mut sensor = sensor;
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
    log::info!("Creating detection area");
    Timer::after_millis(1).await;
    let floor_da = DetectionArea {
        area: Area3D {
            range_x: (-230, 230),
            range_y: (100, 300),
            range_z: (-100, 2000),
        },
        sensor: [Some(SensorType::TMF8821), None, None, None, None, None],
        group: 0,
        use_assumed: false,
    };
    let main_da = DetectionArea {
        area: Area3D {
            range_x: (-230, 230),
            range_y: (300, 1600),
            range_z: (-100, 2000),
        },
        sensor: [Some(SensorType::TMF8821), None, None, None, None, None],
        group: 0,
        use_assumed: false,
    };
    let sens = Sensor {
        sensor: SensorType::TMF8821,
        mount_position: (0, 0),
        mount_angle: (FixedPoint::from(0), FixedPoint::from(-30)),
        enabled: true,
        error: true,
    };
    Timer::after_millis(1).await;
    loop {
        // log::info!("AAAAAAAAAA");
        let res = sensor.single_measurement().await;
        // log::info!("Took single measurement");
        // Timer::after_millis(1).await;
        if let Ok(res) = res {
            let datas = res.to_sensor_data();
            // log::info!("got datas");
            // Timer::after_millis(1).await;
            // for s in datas{
            //     // log::info!(s);
            //     log::info!("{s:?}");
            //     if let Some(p) = s{
            //         // p.print().await;
            //         log::info!("{:?}", p.to_point());
            //     }
            // }
            let mut points = [const { None }; 36];
            for i in 0..18 {
                if let Some(d) = datas[i] {
                    points[i] = Some(d.to_point(&sens))
                }
            }
            log::info!("");
            log::info!("Data group: ");
            log::info!("Floor");
            for s in points.iter().flatten() {
                // //     // log::info!("{s:?}");
                // //     // Timer::after_millis(1).await;
                // //     // s.print().await;
                // //     Timer::after_millis(1).await;
                let _ = floor_da.severity_from_points(&[Some(*s)]);
                // log::info!("{dets:?}");
                // //     // panic_wait("msg").await;
            }
            log::info!("\nMain");
            for s in points.iter().flatten() {
                // //     // log::info!("{s:?}");
                // //     // Timer::after_millis(1).await;
                // //     // s.print().await;
                // //     Timer::after_millis(1).await;
                let _ = main_da.severity_from_points(&[Some(*s)]);
                // log::info!("{dets:?}");
                // //     // panic_wait("msg").await;
            }

            // let dets = da.severity_from_points(&points);
            // log::info!("");
            // // log::info!("{points:?}");
            // log::info!("{dets:?}");
            // if dets.unwrap_or((5000, 5000)).0 < 2000{
            //     log::info!("ALERT");
            // }
            // log::info!("{res}");
            // panic_wait("msg").await;
        } else {
            log::info!("Failed to get sensor measurement!!!");
            Timer::after_millis(1000).await;
        }

        Timer::after_millis(250).await;
        // panic_wait("exited").await;
    }
}
