use core::cmp::min;

use derive_more::Display;
use embassy_rp::gpio::Output;
use embassy_time::Instant;
use embedded_hal_async::{self as hal, delay::DelayNs};

use hal::i2c::SevenBitAddress;

use crate::sensors::core::SensorType;
use crate::sensors::core::utils::SensorData;
use crate::sensors::tmf8821::config::{SPAD, TofActiveRange};
use crate::sensors::tmf8821::constants::*;
use crate::sensors::tmf8821::{config::TmfConfiguration, constants::TOF_BIN_IMAGE};

#[derive(Debug)]
pub enum Error<I2cE>
where
    I2cE: hal::i2c::Error,
{
    /// I²C bus error
    I2c(I2cE),
    /// The QMP6988 chip has not been detected
    ChipNotDetected,
    /// The computed CRC and the one sent by the device mismatch
    BadCrc,
    NoAck,
    ParseError,
}

impl<I2cE> From<I2cE> for Error<I2cE>
where
    I2cE: hal::i2c::Error,
{
    fn from(value: I2cE) -> Self {
        Error::I2c(value)
    }
}
#[derive(Debug, Display)]
// #[display("result_number: {:?}, temperature: {:?}, number_valid_results: {:?}, ambient_light: {:?}, photon_count: {:?}, reference_count: {:?}, sys_tick: {:?},\r\ndistances: {:?}, \r\nconfidences: {:?}", result_number, temperature, number_valid_results, ambient_light, photon_count, reference_count, sys_tick, distances, confidences)]
#[display(
    "{:?}-{} {} {} {:?} {:?}",
    spad,
    ambient_light,
    x,
    y,
    distances,
    confidences
)]
pub struct Measurement {
    result_number: u8,
    temperature: u8,
    number_valid_results: u8,
    ambient_light: u32,
    photon_count: u32,
    reference_count: u32,
    sys_tick: u32,
    distances: [u16; 18],
    confidences: [u8; 18],
    x: u8,
    y: u8,
    spad: SPAD,
}
impl Measurement {
    pub fn to_sensor_data(&self) -> [Option<SensorData>; 18] {
        let mut res: [Option<SensorData>; 18] = [const { None }; 18];
        let vp = self.spad.as_view();
        for (i, item) in res.iter_mut().enumerate().take(self.distances.iter().len()) {
            let m = self.spad.index_as_xy(i as u8);
            if let Some((x, y)) = m {
                let segment_vp = vp.get_subdivision(x, y);
                let sd = SensorData {
                    viewport: segment_vp,
                    sensor: SensorType::TMF8821,
                    range: Some(self.distances[i] as u32),
                    confidence: self.confidences[i] as u32,
                    max_range: 0,
                };
                if self.distances[i] > 0 {
                    *item = Some(sd);
                }
            }
        }
        res
    }
    pub fn from_data(data: &[u8], mode: SPAD) -> Option<Measurement> {
        let offset = TMF882X_REG_CONFIG_RESULT;
        let mut distances: [u16; 18] = [0; 18];
        let mut confidences: [u8; 18] = [0; 18];

        if data[(TMF882X_REG_CONFIG_RESULT - offset) as usize] != TMF882X_MEASUREMENT_RESULT {
            return None;
        }
        for i in 0..18 {
            let index = (i * 3) + 0x38 - offset as usize;
            confidences[i] = data[index];
            distances[i] = data[index + 1] as u16 + ((data[index + 2] as u16) << 8);
        }
        let ambient_light_results = &data[(TMF882X_REG_AMBIENT_LIGHT_0 - offset) as usize
            ..(TMF882X_REG_AMBIENT_LIGHT_3 - offset + 1) as usize];
        let photon_count_results = &data[(TMF882X_REG_PHOTON_COUNT_0 - offset) as usize
            ..(TMF882X_REG_PHOTON_COUNT_3 - offset + 1) as usize];
        let reference_count_results = &data[(TMF882X_REG_REFERENCE_COUNT_0 - offset) as usize
            ..(TMF882X_REG_REFERENCE_COUNT_3 - offset + 1) as usize];
        let sys_tick_results = &data[(TMF882X_REG_SYS_TICK_0 - offset) as usize
            ..(TMF882X_REG_SYS_TICK_3 - offset + 1) as usize];

        Some(Measurement {
            result_number: data[(TMF882X_REG_RESULT_NUMBER - offset) as usize],
            temperature: data[(TMF882X_REG_TEMPERATURE - offset) as usize],
            number_valid_results: data[(TMF882X_REG_NUMBER_VALID_RESULTS - offset) as usize],
            ambient_light: ambient_light_results[0] as u32
                + ((ambient_light_results[1] as u32) << 8)
                + ((ambient_light_results[2] as u32) << 16)
                + ((ambient_light_results[3] as u32) << 24),
            photon_count: photon_count_results[0] as u32
                + ((photon_count_results[1] as u32) << 8)
                + ((photon_count_results[2] as u32) << 16)
                + ((photon_count_results[3] as u32) << 24),
            reference_count: reference_count_results[0] as u32
                + ((reference_count_results[1] as u32) << 8)
                + ((reference_count_results[2] as u32) << 16)
                + ((reference_count_results[3] as u32) << 24),
            sys_tick: sys_tick_results[0] as u32
                + ((sys_tick_results[1] as u32) << 8)
                + ((sys_tick_results[2] as u32) << 16)
                + ((sys_tick_results[3] as u32) << 24),
            confidences,
            distances,
            x: mode.divisions().0,
            y: mode.divisions().1,
            spad: mode,
        })
    }
}
pub struct TMF8821<I2C> {
    address: SevenBitAddress,
    delay: embassy_time::Delay,
    i2c: I2C,
    pin: Output<'static>,
    initialized: bool,
    pub config: TmfConfiguration,
}
impl<I2C> TMF8821<I2C>
where
    I2C: hal::i2c::I2c,
{
    /// Initialize a tmf8821 sensor object
    pub async fn new(
        i2c: I2C,
        address: SevenBitAddress,
        delay: embassy_time::Delay,
        pin: Output<'static>,
    ) -> Self {
        Self {
            address,
            delay,
            i2c,
            pin,
            initialized: false,
            config: TmfConfiguration::new_default(),
        }
    }

    /// Setup the sensor, flashing firmware.
    pub async fn init(&mut self) -> Result<(), Error<I2C::Error>> {
        self.pin.set_low();
        self.initialized = false;
        self.delay.delay_ms(10).await;
        self.pin.set_high();
        let read = self.read_reg(TMF882X_REG_ID).await?;
        log::info!("Sensor read reg");
        let res = read & TMF882X_CHIP_ID_VALID_MASK != TMF882X_CHIP_ID;
        if res {
            log::info!("Could not connect to device");
            return Err(Error::ChipNotDetected);
        }
        let instant = Instant::now();
        const MAX_DEVICE_WAKE_WAIT: u64 = 100;
        loop {
            if instant.elapsed().as_millis() > MAX_DEVICE_WAKE_WAIT {
                return Err(Error::ChipNotDetected);
            }
            log::info!("iter {}", instant.elapsed().as_millis());
            let en_reg = self.read_reg(TMF882X_REG_ENABLE).await?;
            if en_reg & 0x41 == 0x41 {
                // log::trace!("Device up");
                break;
            }
            if en_reg & 0x0F == 0b0001 {
                //CPU is currently in the process of initializing HW and SW
                continue;
            } else if en_reg & 0x0F == 0b0010 {
                // device in standby mode
                // log::trace!("Device in standby, waking up");
                self.write_reg(TMF882X_REG_ENABLE, en_reg | 0x01).await?;
            } else if en_reg & 0x0F == 0b0110 {
                // log::trace!("Device in measurement, resetting");
                self.write_reg(TMF882X_REG_CMD_STAT, TMF882X_APP_CMD_STOP)
                    .await?;
            }
            self.delay.delay_ms(1).await;
        }

        log::info!("Device started!!!!");
        let app_id = self.read_reg(TMF882X_REG_APPID).await?;
        if app_id == TMF882X_MODE_APP {
            log::info!("Device already flashed");
            self.delay.delay_ms(6).await;
            let mut res: [u8; 3] = [0; 3];

            self.read_bytes(TMF882X_REG_MINOR, &mut res).await?;
            log::info!("{:?}", res);
            let minor: u8 = res[0];
            let patch: u8 = res[1];
            let build: u8 = res[2];
            log::info!("version {}.{} b{}", minor, patch, build);
            return Ok(());
        }
        let mut res: [u8; 2] = [0; 2];
        self.read_bytes(TMF882X_REG_APPID, &mut res).await?;
        log::info!("Read app id");
        if res[1] != TMF882X_ROM_V2 {
            log::info!("ROM VERSION WRONG");
            return Err(Error::ChipNotDetected);
        }
        let res = [0x29];
        self.write_bl_command(TMF882X_BL_DOWNLOAD_INIT, &res)
            .await?;
        self.check_bl_cmd_executed().await?;

        self.write_bl_command(
            TMF882X_BL_SET_ADDR,
            &[((TMF882X_BL_FW_ADDR >> 8) as u8), TMF882X_BL_FW_ADDR as u8],
        )
        .await?;
        self.check_bl_cmd_executed().await?;

        let mut size_downloaded = 0;
        log::info!("Downloading firmware");
        while size_downloaded < TOF_BIN_IMAGE.len() {
            let chunk: usize = min(
                TMF882X_BL_MAX_FW_DATA as usize,
                TOF_BIN_IMAGE.len() - size_downloaded,
            );
            let data = &TOF_BIN_IMAGE[size_downloaded..size_downloaded + chunk];
            self.write_bl_command(TMF882X_BL_W_RAM, data).await?;
            self.check_bl_cmd_executed().await?;
            size_downloaded += chunk;
            log::info!("{}/{}", size_downloaded, TOF_BIN_IMAGE.len());
        }
        log::info!("Firmware complete");

        self.write_reg(TMF882X_REG_ENABLE, 0x21).await?; // set bit 0 (general enable) and bit 5 (run from RAM)

        self.write_bl_command(TMF882X_BL_RAMREMAP_RESET, &[])
            .await?;
        self.check_bl_cmd_executed().await?;

        let now = Instant::now();
        loop {
            if now.elapsed().as_millis() > 3 {
                break;
            }
            let app_id = self.read_reg(TMF882X_REG_APPID).await?;
            if app_id == TMF882X_MODE_APP {
                self.delay.delay_ms(6).await;
                let mut res: [u8; 3] = [0; 3];

                self.read_bytes(TMF882X_REG_MINOR, &mut res).await?;

                let minor: u8 = res[0];
                let patch: u8 = res[1];
                let build: u8 = res[2];
                log::info!("version {}.{} b{}", minor, patch, build);
                // self.config.set_i2c_addr(self.address);

                return Ok(());
            } else {
                log::info!("{}", app_id);
            }
        }
        log::error!("Failed to init");
        Err(Error::ChipNotDetected)
    }
    pub fn get_address(&self) -> u8 {
        if self.initialized {
            self.address
        } else {
            TMF882X_DEFAULT_I2C_ADDR
        }
    }

    /// Currently not working, sets the address to the one specified in config.
    pub async fn set_addr(&mut self) -> Result<(), Error<I2C::Error>> {
        self.config.set_i2c_addr(self.address);
        // self.write_app_command(TMF882X_APP_CMD_LOAD_CONFIG_PAGE_COMMON).await?;
        // self.check_app_cmd_executed(false, None, None).await?;
        self.delay.delay_ms(100).await;
        self.config.config[12] = 0;
        self.config.config[13] = 0;
        self.config.config[20] = 66 << 1;
        self.config.config[23] = 0;
        self.write_configuration().await?;
        // self.write_reg(TMF882X_REG_SPAD_MAP_ID, SPAD::Map3x6 as u8).await?;
        // self.write_reg(TMF882X_REG_GPIO_0, 0x0).await?;
        // self.write_reg(TMF882X_REG_GPIO_1, 0x0).await?;
        // self.write_reg(TMF882X_REG_I2C_ADDR_CHANGE, 0).await?;
        // self.write_reg(TMF882X_REG_I2C_SLAVE_ADDRESS, self.address<<1).await?;
        self.delay.delay_ms(100).await;
        // self.write_app_command(TMF882X_APP_CMD_WRITE_CONFIG_PAGE).await?;
        // self.check_app_cmd_executed(false, Some(&[0x03]), None).await?;
        // self.delay.delay_ms(100).await;
        // self.write_configuration().await?;
        // self.write_reg(TMF882X_REG_I2C_SLAVE_ADDRESS, self.address<<1).await?;
        // self.write_app_command(TMF882X_APP_CMD_LOAD_CONFIG_PAGE_COMMON).await?;
        // self.check_app_cmd_executed(false, Some(&[0x03]), None).await?;
        // let mut data = [0;24];

        // self.read_bytes(TMF882X_REG_PERIOD_MS_LSB, &mut data).await?;
        // log::error!("ADDR FROM CFG: {}", data[20]>>1);
        // log::error!("AAAAAAA: {}", data[23]);
        // log::info!("READ CONFIG: {:?}", data);
        // log::info!("Wrote config");
        self.write_bl_command(0x21, &[]).await?;

        // for i in 0..255{
        //     let mut a: [u8; 1] = [0];
        //     let r = self.i2c.read(i, &mut a).await;
        //     if r.is_ok(){
        //         log::info!("Responded on: {i}");
        //     }
        // }
        log::info!("Wrote set addr cmd");

        // self.delay.delay_ms(100).await;
        // self.delay.delay_ms(1000).await;
        // self.check_bl_cmd_executed().await?;
        self.delay.delay_ms(100).await;
        for i in 0..255 {
            let mut a: [u8; 1] = [0];
            let r = self.i2c.read(i, &mut a).await;
            if r.is_ok() {
                log::info!("Responded on: {i}");
            }
        }
        self.initialized = true;
        // panic_wait("AAAAAA").await;
        // for i in 0..255{
        //     let mut a: [u8; 1] = [0];
        //     let r = self.i2c.read(i, &mut a).await;
        //     if r.is_ok(){
        //         log::info!("Responded on: {i}");
        //     }
        // }
        log::info!("cmd executed");
        Ok(())
    }

    pub async fn start_measurements(&mut self) -> Result<(), Error<I2C::Error>> {
        // for i in 0..255{
        //     let mut a: [u8; 1] = [0];
        //     let r = self.i2c.read(i, &mut a).await;
        //     if r.is_ok(){
        //         log::info!("Responded on: {i}");
        //     }
        // }
        self.write_reg(TMF882X_REG_INT_ENAB, 0x02).await?;
        self.write_reg(TMF882X_REG_INT_STATUS, 0xFF).await?;
        self.write_app_command(TMF882X_APP_CMD_MEASURE).await?;
        self.check_app_cmd_executed(true, None, None).await?;

        Ok(())
    }
    pub async fn stop_measurements(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_app_command(TMF882X_APP_CMD_STOP).await?;
        self.check_app_cmd_executed(false, None, None).await?;
        Ok(())
    }
    pub async fn single_measurement(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        self.start_measurements().await?;
        let m = self.wait_for_measurement().await?;
        self.stop_measurements().await?;
        Ok(m)
    }
    pub async fn wake(&mut self) -> Result<(), Error<I2C::Error>> {
        let cpu_enable_val = self.read_reg(TMF882X_REG_ENABLE).await?;
        let val = cpu_enable_val & 0x01;
        if val == 0 {
            let cpu_enable_val = cpu_enable_val | 0x01;
            self.write_reg(TMF882X_REG_ENABLE, cpu_enable_val).await?;
            log::info!("Enabled tmf882x");
            self.is_cpu_ready().await?;
        } else {
            log::info!("TMF882X Already Enabled")
        }
        Ok(())
    }
    pub async fn standby(&mut self) -> Result<(), Error<I2C::Error>> {
        let cpu_enable_val = self.read_reg(TMF882X_REG_ENABLE).await?;
        let val = cpu_enable_val & 0x01;
        if val > 0 {
            let cpu_enable_val = cpu_enable_val & !0x01;
            self.write_reg(TMF882X_REG_ENABLE, cpu_enable_val).await?;
            log::info!("Sleeping tmf8821");
        } else {
            log::info!("Already asleep!");
        }
        Ok(())
    }
    /// Writes the current configuration to the sensor.
    pub async fn write_configuration(&mut self) -> Result<(), Error<I2C::Error>> {
        log::info!("1");
        self.write_app_command(TMF882X_APP_CMD_LOAD_CONFIG_PAGE_COMMON)
            .await?;
        log::info!("2");
        // self.check_app_cmd_executed(false, None, None).await?;
        let mut config: [u8; 4] = [0; 4];
        self.read_bytes(TMF882X_REG_CONFIG_RESULT, &mut config)
            .await?;
        let size: u32 = config[2] as u32 + ((config[3] as u32) << 8);
        if !(config[0] == TMF882X_CONFIG_COMMON_CID && size == TMF882X_CONFIG_PAGE_SIZE) {
            log::info!("INCORRECT CONFIG SIZE");
            log::info!("{:?}", config);
            log::info!(
                "{} {} {} {}",
                config[0],
                TMF882X_CONFIG_COMMON_CID,
                { size },
                TMF882X_CONFIG_PAGE_SIZE
            );
        }
        log::info!("3");
        let mut data: [u8; 25] = [0; 25];
        data[0] = TMF882X_REG_PERIOD_MS_LSB;
        data[1..(24 + 1)].copy_from_slice(&self.config.config);
        log::info!("{data:?}");
        self.i2c.write(self.get_address(), &data).await?;
        log::info!("4");
        self.write_app_command(TMF882X_APP_CMD_WRITE_CONFIG_PAGE)
            .await?;
        log::info!("5");
        self.check_app_cmd_executed(false, Some(&[0x03]), None)
            .await?;
        log::info!("6");
        Ok(())
    }
    pub async fn factory_calibration(&mut self) -> Result<(), Error<I2C::Error>> {
        let timeout = 500;
        self.write_app_command(TMF882X_APP_CMD_FACTORY_CALIBRATION)
            .await?;
        self.check_app_cmd_executed(false, None, Some(timeout))
            .await?;

        let calib_status = self.read_reg(TMF882X_REG_CALIBRATION_STATUS_FC).await?;
        if calib_status != 0x00 {
            return Err(Error::NoAck);
        }
        self.write_app_command(TMF882X_APP_CMD_LOAD_CONFIG_PAGE_FACTORY_CALIB)
            .await?;
        self.check_app_cmd_executed(false, None, Some(50)).await?;
        let mut raw_page: [u8; TMF882X_CALIBRATION_PAGE_SIZE + 4] =
            [0; TMF882X_CALIBRATION_PAGE_SIZE + 4];
        self.read_bytes(TMF882X_REG_CONFIG_RESULT, raw_page.as_mut_slice())
            .await?;

        Ok(())
    }
    /// When measurement is enabled, wait for one and then return it.
    pub async fn wait_for_measurement(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        let now = Instant::now();
        const MAX_MEASUREMENT_WAIT: u64 = 100;
        loop {
            if now.elapsed().as_millis() > MAX_MEASUREMENT_WAIT {
                return Err(Error::NoAck);
            }
            let interrupts = self.read_reg(TMF882X_REG_INT_STATUS).await?;
            if (interrupts & 0x02) != 0 {
                break;
            }
        }
        let mut measurement_data: [u8; _] = [0; TMF882X_MEASUREMENT_SIZE];
        self.write_reg(TMF882X_REG_INT_STATUS, 0x02).await?;
        self.read_bytes(TMF882X_REG_CONFIG_RESULT, &mut measurement_data)
            .await?;
        let res = self.read_reg(TMF882X_REG_MEASURE_STATUS).await?;
        if res != 0x00 {
            return Err(Error::NoAck);
        }
        let mes = Measurement::from_data(&measurement_data, self.config.spad);
        match mes {
            Some(s) => Ok(s),
            None => Err(Error::ParseError),
        }
    }
    /// Read the specified register
    async fn read_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let mut result = [0u8];
        self.i2c
            .write_read(self.get_address(), &[reg], &mut result)
            .await?;
        Ok(result[0])
    }
    /// Writes the provided value to the specific register.
    async fn write_reg(&mut self, reg: u8, data: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c.write(self.get_address(), &[reg, data]).await?;
        Ok(())
    }
    /// Reads a number of bytes into the provided buffer.
    async fn read_bytes(&mut self, reg: u8, res: &mut [u8]) -> Result<(), Error<I2C::Error>> {
        // let mut res = &target;
        self.i2c.write_read(self.get_address(), &[reg], res).await?;
        log::info!("Successful read from {}", self.get_address());
        Ok(())
    }
    fn sum(res: &[u8]) -> u8 {
        let mut a: u8 = 0;
        for s in res {
            a += *s;
        }
        a
    }
    async fn write_app_command(&mut self, cmd: u8) -> Result<(), Error<I2C::Error>> {
        self.write_reg(TMF882X_REG_CMD_STAT, cmd).await?;
        Ok(())
    }
    async fn check_app_cmd_executed(
        &mut self,
        allow_accept: bool,
        ignore: Option<&[u8]>,
        timeout: Option<u64>,
    ) -> Result<(), Error<I2C::Error>> {
        let timeout: u64 = timeout.unwrap_or(100);
        let ignore = ignore.unwrap_or(&[]);
        let now = Instant::now();
        while now.elapsed().as_millis() < timeout {
            let status = self.read_reg(TMF882X_REG_CMD_STAT).await?;
            if status == TMF882X_CMD_STAT_OK {
                return Ok(());
            }
            if status == TMF882X_CMD_STAT_ACCEPTED {
                if allow_accept {
                    return Ok(());
                } else {
                    continue;
                }
            } else if ignore.contains(&status) {
                return Ok(());
            }
            if status > 0x10 {
                continue;
            }
            log::info!("BAD VALUE {status}");
        }
        Err(Error::NoAck)
    }
    async fn is_cpu_ready(&mut self) -> Result<(), Error<I2C::Error>> {
        const CPU_READY_WAIT: u64 = 1000;
        let now = Instant::now();
        loop {
            if now.elapsed().as_millis() > CPU_READY_WAIT {
                break;
            }
            let res = self.read_reg(TMF882X_REG_ENABLE).await?;
            if res & 0x40 > 0 {
                return Ok(());
            }
            self.delay.delay_ms(1).await;
        }
        Err(Error::ChipNotDetected)
    }
    async fn write_bl_command(&mut self, cmd: u8, res: &[u8]) -> Result<(), Error<I2C::Error>> {
        let checksum: u8 = (cmd + res.len() as u8 + Self::sum(res)) ^ 0xFF;
        let mut temp: [u8; 260] = [0; 260];
        log::info!("Checksum: {checksum}");
        temp[0] = TMF882X_REG_BL_CMD_STAT;
        temp[1] = cmd;
        temp[2] = (res.len()) as u8;
        temp[3..(res.len() + 3)].copy_from_slice(res);
        temp[3 + res.len()] = checksum;
        let a = &temp[0..res.len() + 4];
        self.i2c.write(self.get_address(), a).await?;
        Ok(())
    }
    async fn check_bl_cmd_executed(&mut self) -> Result<(), Error<I2C::Error>> {
        let now = Instant::now();

        log::info!("addr for check bl cmd {}", self.get_address());
        while now.elapsed().as_millis() < 100 {
            let mut status: [u8; 3] = [0xff; 3];
            let _ = self.read_bytes(TMF882X_REG_BL_CMD_STAT, &mut status).await;

            if status[1] == TMF882X_CMD_STAT_ACCEPTED {
                continue;
            } else if status[1] == TMF882X_CMD_STAT_OK {
                return Ok(());
            } else {
                log::info!("Status: {:?}", status);
            }
        }
        for i in 0..255 {
            let mut a: [u8; 1] = [0];
            let r = self.i2c.read(i, &mut a).await;
            if r.is_ok() {
                log::info!("Responded on: {i}");
            }
        }
        Err(Error::NoAck)
    }

    pub async fn set_range(&mut self, range: TofActiveRange) -> Result<(), Error<I2C::Error>> {
        if range == TofActiveRange::LONG {
            self.write_app_command(TMF882X_ACTIVE_RANGE_LONG).await?;
            self.check_app_cmd_executed(false, None, None).await?;
            let res = self.read_reg(TMF882X_REG_ACTIVE_RANGE).await?;
            if res != TMF882X_ACTIVE_RANGE_LONG {
                return Err(Error::NoAck);
            }
            return Ok(());
        } else if range == TofActiveRange::SHORT {
            self.write_app_command(TMF882X_ACTIVE_RANGE_SHORT).await?;
            self.check_app_cmd_executed(false, None, None).await?;
            let res = self.read_reg(TMF882X_REG_ACTIVE_RANGE).await?;
            if res != TMF882X_ACTIVE_RANGE_SHORT {
                return Err(Error::NoAck);
            }
            return Ok(());
        }
        Err(Error::ParseError)
    }
}
