use core::cmp::min;

use derive_more::Display;
use embassy_time::Instant;
use embedded_hal_async::{self as hal, delay::DelayNs};

use hal::i2c::SevenBitAddress;

use crate::sensors::tmf8821::constants::*;
// use crate::tmf8821::device::Err3::{NoConn, NoInit};
use crate::sensors::tmf8821::{config::TmfConfiguration, constants::TOF_BIN_IMAGE};
pub enum TofActiveRange {
    LONG,
    SHORT,
}

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
#[display("{} {:?} {:?}", temperature, distances, confidences)]
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
}
impl Measurement {
    pub fn from_data(data: &[u8]) -> Option<Measurement> {
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
        })
    }
}
pub struct TMF8821<I2C> {
    address: SevenBitAddress,
    delay: embassy_time::Delay,
    i2c: I2C,
    pub config: TmfConfiguration,
}
impl<I2C> TMF8821<I2C>
where
    I2C: hal::i2c::I2c,
{
    pub async fn new(
        i2c: I2C,
        address: SevenBitAddress,
        delay: embassy_time::Delay,
    ) -> Result<Self, Error<I2C::Error>> {
        let mut device = Self {
            address,
            delay,
            i2c,
            config: TmfConfiguration::new_default(),
        };
        let read = device.read_reg(TMF882X_REG_ID).await?;
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
            let en_reg = device.read_reg(TMF882X_REG_ENABLE).await?;
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
                device.write_reg(TMF882X_REG_ENABLE, en_reg | 0x01).await?;
            } else if en_reg & 0x0F == 0b0110 {
                // log::trace!("Device in measurement, resetting");
                device
                    .write_reg(TMF882X_REG_CMD_STAT, TMF882X_APP_CMD_STOP)
                    .await?;
            }
            device.delay.delay_ms(1).await;
        }

        log::info!("Device started!!!!");
        let app_id = device.read_reg(TMF882X_REG_APPID).await?;
        if app_id == TMF882X_MODE_APP {
            log::info!("Device already flashed");
            device.delay.delay_ms(6).await;
            let mut res: [u8; 3] = [0; 3];

            device.read_bytes(TMF882X_REG_MINOR, &mut res).await?;
            log::info!("{:?}", res);
            let minor: u8 = res[0];
            let patch: u8 = res[1];
            let build: u8 = res[2];
            log::info!("version {}.{} b{}", minor, patch, build);
            return Ok(device);
        }
        let mut res: [u8; 2] = [0; 2];
        device.read_bytes(TMF882X_REG_APPID, &mut res).await?;
        log::info!("{:?}", res);

        log::info!("ROM VERSION: {}", res[1]);
        if res[1] != TMF882X_ROM_V2 {
            log::info!("ROM VERSION WRONG");
            return Ok(device);
        }

        log::info!("SOMEHOW WE GOOD");
        // if res[2]
        let res = [0x29];
        device
            .write_bl_command(TMF882X_BL_DOWNLOAD_INIT, &res)
            .await?;
        device.check_bl_cmd_executed().await?;

        device
            .write_bl_command(
                TMF882X_BL_SET_ADDR,
                &[((TMF882X_BL_FW_ADDR >> 8) as u8), TMF882X_BL_FW_ADDR as u8],
            )
            .await?;
        device.check_bl_cmd_executed().await?;

        let mut size_downloaded = 0;
        log::info!("Downloading firmware");
        while size_downloaded < TOF_BIN_IMAGE.len() {
            let chunk: usize = min(
                TMF882X_BL_MAX_FW_DATA as usize,
                TOF_BIN_IMAGE.len() - size_downloaded,
            );
            let data = &TOF_BIN_IMAGE[size_downloaded..size_downloaded + chunk];
            device.write_bl_command(TMF882X_BL_W_RAM, data).await?;
            device.check_bl_cmd_executed().await?;
            size_downloaded += chunk;
            log::info!("{}/{}", size_downloaded, TOF_BIN_IMAGE.len());
        }
        log::info!("Finished writing fw");
        device.write_reg(TMF882X_REG_ENABLE, 0x21).await?; // set bit 0 (general enable) and bit 5 (run from RAM)
        device
            .write_bl_command(TMF882X_BL_RAMREMAP_RESET, &[])
            .await?;

        let now = Instant::now();
        while now.elapsed().as_millis() < 3 {
            let app_id = device.read_reg(TMF882X_REG_APPID).await?;
            if app_id == TMF882X_MODE_APP {
                device.delay.delay_ms(6).await;
                let mut res: [u8; 3] = [0; 3];

                device.read_bytes(TMF882X_REG_MINOR, &mut res).await?;
                log::info!("{:?}", res);

                let minor: u8 = res[0];
                let patch: u8 = res[1];
                let build: u8 = res[2];
                log::info!("version {}.{} b{}", minor, patch, build);
                return Ok(device);
            } else {
                log::info!("{}", app_id);
            }
        }
        panic!("");
    }

    pub async fn read_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let mut result = [0u8];
        self.i2c
            .write_read(self.address, &[reg], &mut result)
            .await?;
        Ok(result[0])
    }
    pub async fn start_measurements(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_reg(TMF882X_REG_INT_ENAB, 0x02).await?;
        self.write_reg(TMF882X_REG_INT_STATUS, 0xFF).await?;
        self.write_app_command(TMF882X_APP_CMD_MEASURE).await?;
        self.check_app_cmd_executed(true, None).await?;

        Ok(())
    }
    pub async fn write_configuration(&mut self) {
        self.write_app_command(TMF882X_APP_CMD_LOAD_CONFIG_PAGE_COMMON)
            .await
            .unwrap();
        self.check_app_cmd_executed(false, None).await.unwrap();
        let mut data: [u8; 25] = [0; 25];
        data[0] = TMF882X_REG_PERIOD_MS_LSB;
        data[1..(24 + 1)].copy_from_slice(&self.config.config);

        self.i2c.write(self.address, &data).await.unwrap();
        self.write_app_command(TMF882X_APP_CMD_WRITE_CONFIG_PAGE)
            .await
            .unwrap();
        self.check_app_cmd_executed(false, Some(&[0x03]))
            .await
            .unwrap();
    }
    pub async fn wait_for_measurement(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        let now = Instant::now();
        const MAX_MEASUREMENT_WAIT: u64 = 100;
        loop {
            if now.elapsed().as_millis() > MAX_MEASUREMENT_WAIT {
                return Err(Error::NoAck);
            }
            let interrupts = self.read_reg(TMF882X_REG_INT_STATUS).await.unwrap();
            if (interrupts & 0x02) != 0 {
                break;
            }
        }
        let mut measurement_data: [u8; _] = [0; TMF882X_MEASUREMENT_SIZE];
        self.write_reg(TMF882X_REG_INT_STATUS, 0x02).await.unwrap();
        self.read_bytes(TMF882X_REG_CONFIG_RESULT, &mut measurement_data)
            .await?;
        let res = self.read_reg(TMF882X_REG_MEASURE_STATUS).await?;
        if res != 0x00 {
            return Err(Error::NoAck);
        }
        let mes = Measurement::from_data(&measurement_data);
        match mes {
            Some(s) => Ok(s),
            None => Err(Error::ParseError),
        }
    }
    pub async fn write_reg(&mut self, reg: u8, data: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c.write(self.address, &[reg, data]).await?;
        Ok(())
    }
    pub async fn read_bytes(&mut self, reg: u8, res: &mut [u8]) -> Result<(), Error<I2C::Error>> {
        // let mut res = &target;
        self.i2c.write_read(self.address, &[reg], res).await?;
        Ok(())
    }
    fn sum(res: &[u8]) -> u8 {
        let mut a: u8 = 0;
        for s in res {
            a += *s;
        }
        a
    }
    pub async fn write_app_command(&mut self, cmd: u8) -> Result<(), Error<I2C::Error>> {
        self.write_reg(TMF882X_REG_CMD_STAT, cmd).await?;
        Ok(())
    }
    pub async fn check_app_cmd_executed(
        &mut self,
        allow_accept: bool,
        ignore: Option<&[u8]>,
    ) -> Result<(), Error<I2C::Error>> {
        let ignore = ignore.unwrap_or(&[]);
        let now = Instant::now();
        while now.elapsed().as_millis() < 100 {
            let status = self.read_reg(TMF882X_REG_CMD_STAT).await.expect("msg");
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
    pub async fn write_bl_command(&mut self, cmd: u8, res: &[u8]) -> Result<(), Error<I2C::Error>> {
        let checksum: u8 = cmd + res.len() as u8 + Self::sum(res);
        let mut temp: [u8; 260] = [0; 260];
        log::info!("Checksum: {checksum}");
        temp[0] = TMF882X_REG_BL_CMD_STAT;
        temp[1] = cmd;
        temp[2] = (res.len()) as u8;
        temp[3..(res.len() + 3)].copy_from_slice(res);

        temp[3 + res.len()] = checksum;

        let a = &temp[0..res.len() + 4];
        self.i2c.write(self.address, a).await?;
        Ok(())
    }
    pub async fn check_bl_cmd_executed(&mut self) -> Result<(), Error<I2C::Error>> {
        const MAX_ACK: u64 = 100;
        let now = Instant::now();
        while now.elapsed().as_millis() < MAX_ACK {
            let mut status: [u8; 3] = [0; 3];
            let _ = self.read_bytes(TMF882X_REG_BL_CMD_STAT, &mut status).await;
            if status[1] == TMF882X_CMD_STAT_ACCEPTED {
                // Command
                continue;
            }
             if status[1] == TMF882X_CMD_STAT_OK {
                // Command
                return Ok(());
            } 
                log::info!("Status: {status:?}");
        }
        Err(Error::NoAck)
    }
}
