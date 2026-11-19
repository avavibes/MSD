use core::{any::Any, cmp::min};

use derive_more::Display;
use embassy_time::{Delay, Instant};
use embedded_hal_async::{self as hal, delay::DelayNs};

use hal::i2c::{Operation, SevenBitAddress};

use crate::tmf8821::constants::*;
// use crate::tmf8821::device::Err3::{NoConn, NoInit};
use crate::tmf8821::{config::TMF882X_Configuration, constants::tof_bin_image};
pub enum ACTIVE_RANGE {
    LONG,
    SHORT,
}

// #[derive(Debug)]
// pub enum Err3 {
//     NoAck,
//     NoInit,
//     NoConn,
//     I2CErr(I2C::Error)
// }
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
#[display("{} {:?} {:?}", ambient_light, distances, confidences)]
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
        let offset = _tmf882X_REG_CONFIG_RESULT;
        let mut distances: [u16; 18] = [0; 18];
        let mut confidences: [u8; 18] = [0; 18];

        if data[(_tmf882X_REG_CONFIG_RESULT - offset) as usize] != _tmf882X_MEASUREMENT_RESULT {
            // log::info!("{}",data[(_tmf882X_REG_CONFIG_RESULT-offset) as usize]);
            return None;
        }
        for i in 0..18 {
            let index = (i * 3) + 0x38 - offset as usize;
            confidences[i] = data[index];
            distances[i] = data[index + 1] as u16 + ((data[index + 2] as u16) << 8);
        }
        let ambient_light_results = &data[(_tmf882X_REG_AMBIENT_LIGHT_0 - offset) as usize
            ..(_tmf882X_REG_AMBIENT_LIGHT_3 - offset + 1) as usize];
        let photon_count_results = &data[(_tmf882X_REG_PHOTON_COUNT_0 - offset) as usize
            ..(_tmf882X_REG_PHOTON_COUNT_3 - offset + 1) as usize];
        let reference_count_results = &data[(_tmf882X_REG_REFERENCE_COUNT_0 - offset) as usize
            ..(_tmf882X_REG_REFERENCE_COUNT_3 - offset + 1) as usize];
        let sys_tick_results = &data[(_tmf882X_REG_SYS_TICK_0 - offset) as usize
            ..(_tmf882X_REG_SYS_TICK_3 - offset + 1) as usize];

        // log::info!("{:?}", ambient_light_results);
        return Some(Measurement {
            result_number: data[(_tmf882X_REG_RESULT_NUMBER - offset) as usize],
            temperature: data[(_tmf882X_REG_TEMPERATURE - offset) as usize],
            number_valid_results: data[(_tmf882X_REG_NUMBER_VALID_RESULTS - offset) as usize],
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
            confidences: confidences,
            distances: distances,
        });
        // return None
        // Measurement { result_number: (), temperature: (), number_valid_results: (), ambient_light: (), photon_count: (), reference_count: (), sys_tick: (), distances: (), confidences: () }
    }
}
pub struct TMF8821<I2C> {
    address: SevenBitAddress,
    delay: embassy_time::Delay,
    i2c: I2C,
    // i2c: &'a i2c::I2c<'a, embassy_rp::peripherals::I2C0, i2c::Blocking>,
    pub config: TMF882X_Configuration,
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
            config: TMF882X_Configuration::new_default(),
        };
        let read = device.read_reg(_tmf882X_REG_ID).await?;
        // if let Err(e) = read {
        //     log::error!("Failed to read reg id");
        //     return Err(Error::);
        // }
        // let read = read.unwrap();
        let res = read & _tmf882X_CHIP_ID_VALID_MASK != _tmf882X_CHIP_ID;
        if res {
            log::info!("Could not connect to device");
            return Err(Error::ChipNotDetected);
        }
        let instant = Instant::now();

        while instant.elapsed().as_millis() < 100 {
            log::info!("iter {}", instant.elapsed().as_millis());
            let en_reg = device.read_reg(_tmf882X_REG_ENABLE).await?;
            if en_reg & 0x41 == 0x41 {
                // log::trace!("Device up");
                break;
            }
            if en_reg & 0x0F == 0b0001 {
                //CPU is currently in the process of initializing HW and SW
                continue;
            } else if en_reg & 0x0F == 0b0001 {
                // device in standby mode
                // log::trace!("Device in standby, waking up");
                device
                    .write_reg(_tmf882X_REG_ENABLE, en_reg | 0x01)
                    .await?;

            } else if en_reg & 0x0F == 0b0110 {
                // log::trace!("Device in measurement, resetting");
               device
                    .write_reg(_tmf882X_REG_CMD_STAT, _tmf882X_APP_CMD_STOP)
                    .await?
                    ;
            }
            device.delay.delay_ms(1).await;
        }
        if instant.elapsed().as_millis() >= 100 {
            log::info!("Device failed init");
            panic!("Failed to init");
        }
        log::info!("Device started!!!!");
        let app_id = device.read_reg(_tmf882X_REG_APPID).await.unwrap();
        if app_id == _tmf882X_MODE_APP {
            log::info!("Device already flashed");
            device.delay.delay_ms(6).await;
            let mut res: [u8; 3] = [0; 3];

            let r2 = device.read_bytes(_tmf882X_REG_MINOR, &mut res).await;
            log::info!("{:?}", res);
            match r2 {
                Ok(s) => s,
                Err(e) => {
                    log::info!("AAAAAA");
                    return Ok(device);
                }
            };
            let minor: u8 = res[0];
            let patch: u8 = res[1];
            let build: u8 = res[2];
            log::info!("version {}.{} b{}", minor, patch, build);
            return Ok(device);
        }
        let mut res: [u8; 2] = [0; 2];
        let r2 = device.read_bytes(_tmf882X_REG_APPID, &mut res).await;
        log::info!("{:?}", res);
        match r2 {
            Ok(s) => s,
            Err(e) => {
                log::info!("BBBBB");
                return Ok(device);
            }
        };

        log::info!("ROM VERSION: {}", res[1]);
        if res[1] != _tmf882X_ROM_V2 {
            log::info!("ROM VERSION WRONG");
            return Ok(device);
        }

        log::info!("SOMEHOW WE GOOD");
        // if res[2]
        let res = [0x29];
        let r2 = device
            .write_bl_command(_tmf882X_BL_DOWNLOAD_INIT, &res)
            .await;
        match r2 {
            Ok(s) => s,
            Err(e) => {
                log::info!("BBBBB");
                return Ok(device);
            }
        };
        let a = device.check_bl_cmd_executed().await;

        // match res{}
        if a.is_err() {
            log::info!("BADDDDD");
        }
        if a.is_ok() {
            log::info!("GOOD");
        }
        // panic!();
        // return Ok(device);
        let r2 = device
            .write_bl_command(
                _tmf882X_BL_SET_ADDR,
                &[
                    (_tmf882X_BL_FW_ADDR >> 8) as u8 & 0xFF,
                    _tmf882X_BL_FW_ADDR as u8 & 0xFF,
                ],
            )
            .await;
        match r2 {
            Ok(s) => s,
            Err(e) => {
                log::info!("cmd failed for set fw addr");
                return Ok(device);
            }
        };
        let a = device.check_bl_cmd_executed().await;

        // match res{}
        if a.is_err() {
            log::info!("ack failed for set fw addr");
        }
        if a.is_ok() {
            log::info!("ack succeeded for set fw addr");
        }
        let mut size_downloaded = 0;
        log::info!("Downloading firmware");
        while size_downloaded < tof_bin_image.len() {
            let chunk: usize = min(
                _tmf882X_BL_MAX_FW_DATA as usize,
                tof_bin_image.len() - size_downloaded,
            );
            let data = &tof_bin_image[size_downloaded..size_downloaded + chunk];
            let r2 = device.write_bl_command(_tmf882X_BL_W_RAM, data).await;
            match r2 {
                Ok(s) => s,
                Err(e) => {
                    log::info!("Error in firmware flash");
                    return Ok(device);
                }
            };
            let r2 = device.check_bl_cmd_executed().await;
            match r2 {
                Ok(s) => s,
                Err(e) => {
                    log::info!("noack in firmware flash");
                    return Ok(device);
                }
            };
            size_downloaded += chunk;
            log::info!("{}/{}", size_downloaded, tof_bin_image.len());
        }
        log::info!("Finished writing fw");
        device.write_reg(_tmf882X_REG_ENABLE, 0x21).await.unwrap(); // set bit 0 (general enable) and bit 5 (run from RAM)
        device
            .write_bl_command(_tmf882X_BL_RAMREMAP_RESET, &[])
            .await
            .unwrap();

        let now = Instant::now();
        while now.elapsed().as_millis() < 3 {
            let app_id = device.read_reg(_tmf882X_REG_APPID).await.unwrap();
            if app_id == _tmf882X_MODE_APP {
                device.delay.delay_ms(6).await;
                let mut res: [u8; 3] = [0; 3];

                let r2 = device.read_bytes(_tmf882X_REG_MINOR, &mut res).await;
                log::info!("{:?}", res);
                match r2 {
                    Ok(s) => s,
                    Err(e) => {
                        log::info!("AAAAAA");
                        return Ok(device);
                    }
                };
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

        // Ok(device)
    }
    // pub async fn set_active_range(&mut self, range: ACTIVE_RANGE) -> Result<(), Error<I2C::Error>>{
    //     match
    //     Ok(())
    // }
    pub async fn read_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let mut result = [0u8];
        let res = self.i2c.write_read(self.address, &[reg], &mut result).await;
        match res {
            Ok(_) => Ok(result[0]),
            Err(e) => {
                log::error!("{:?}", e);
                return Err(Error::I2c(e));
            }
        }
        
    }
    pub async fn start_measurements(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_reg(_tmf882X_REG_INT_ENAB, 0x02).await?;
        self.write_reg(_tmf882X_REG_INT_STATUS, 0xFF).await?;
        self.write_app_command(_tmf882X_APP_CMD_MEASURE).await?;
        let res = self.check_app_cmd_executed(true, None).await;
        if res.is_err() {
            log::info!("COULD NOT START MEASUREMENTS");
        }

        Ok(())
    }
    pub async fn write_configuration(&mut self) {
        self.write_app_command(_tmf882X_APP_CMD_LOAD_CONFIG_PAGE_COMMON)
            .await
            .unwrap();
        self.check_app_cmd_executed(false, None).await.unwrap();
        let mut data: [u8; 25] = [0; 25];
        data[0] = _tmf882X_REG_PERIOD_MS_LSB;
        for i in 0..24 {
            data[i + 1] = self.config.config[i];
        }

        self.i2c.write(self.address, &data).await.unwrap();
        self.write_app_command(_tmf882X_APP_CMD_WRITE_CONFIG_PAGE)
            .await
            .unwrap();
        self.check_app_cmd_executed(false, Some(&[0x03]))
            .await
            .unwrap();
    }
    pub async fn wait_for_measurement(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        let now = Instant::now();
        while now.elapsed().as_millis() < 100 {
            let interrupts = self.read_reg(_tmf882X_REG_INT_STATUS).await.unwrap();
            if (interrupts & 0x02) != 0 {
                break;
            }
        }
        if now.elapsed().as_millis() > 100 {
            return Err(Error::NoAck);
        }
        let mut measurement_data: [u8; _] = [0; _tmf882X_MEASUREMENT_SIZE];
        // Ok(Measurement::from_data(self, data))
        self.write_reg(_tmf882X_REG_INT_STATUS, 0x02).await.unwrap();
        let res = self
            .read_bytes(_tmf882X_REG_CONFIG_RESULT, &mut measurement_data)
            .await
            .unwrap();
        let res = self.read_reg(_tmf882X_REG_MEASURE_STATUS).await.unwrap();
        if res != 0x00 {
            return Err(Error::NoAck);
        }
        return Ok(Measurement::from_data(&measurement_data).unwrap());
    }
    pub async fn write_reg(&mut self, reg: u8, data: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c
            .write(self.address, &[reg, data])
            .await?
            ;
        Ok(())
        // match res{
        //     Ok(s) => Ok(()),
        //     Err(e) => Err(Err3::I2CErr()),
        // }
    }
    // pub async fn write_bytes(&mut self, reg: u8, res:&[u8]){
    //     self.i2c.write(self.address, write)
    // }
    pub async fn read_bytes(&mut self, reg: u8, res: &mut [u8]) -> Result<(), Error<I2C::Error>> {
        // let mut res = &target;
        self.i2c.write_read(self.address, &[reg], res).await?;
        Ok(())
    }
    fn sum(res: &[u8]) -> u8 {
        let mut a: u8 = 0;
        for s in res {
            a += *s as u8;
        }
        a
    }
    pub async fn write_app_command(&mut self, cmd: u8) -> Result<(), Error<I2C::Error>> {
        self.write_reg(_tmf882X_REG_CMD_STAT, cmd).await?;
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
            let status = self.read_reg(_tmf882X_REG_CMD_STAT).await.expect("msg");
            if status == _tmf882X_CMD_STAT_OK {
                return Ok(());
            }
            if status == _tmf882X_CMD_STAT_ACCEPTED {
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
        let checksum: u8 =
            (cmd as u8 + res.len() as u8 + Self::sum(res)) & (0x000000FF as u8) ^ 0xFF;
        let checksum = checksum as u8;
        let mut temp: [u8; 260] = [0; 260];
        log::info!("Checksum: {checksum}");
        temp[0] = _tmf882X_REG_BL_CMD_STAT;
        temp[1] = cmd;
        temp[2] = (res.len()) as u8;
        for i in 0..res.len() {
            temp[3 + i] = res[i];
        }

        // log::info!("Data: {:?}", &temp[0..110]);
        temp[3 + res.len()] = checksum;

        // log::info!("Data: {:?}", &temp[0..110]);
        let a = &temp[0..res.len() + 4];
        // log::info!("{:?}", a);
        self.i2c.write(self.address, a).await?;
        // let a = &[cmd, (res.len()) as u8] + res + &[checksum];
        Ok(())
    }
    pub async fn check_bl_cmd_executed(&mut self) -> Result<(), Error<I2C::Error>> {
        let now = Instant::now();
        while now.elapsed().as_millis() < 100 {
            let mut status: [u8; 3] = [0; 3];
            let _ = self.read_bytes(_tmf882X_REG_BL_CMD_STAT, &mut status).await;
            log::info!("Status: {:?}", status);
            // if status[0] == _tmf882X_CMD_STAT_OK{
            //     log::info!("Command okay");
            //     return Ok(())
            // }
            // else if status[0] == _tmf882X_CMD_STAT_ACCEPTED{
            //     log::info!("Command accepted");
            //     continue;
            // }
            // else if status[2] == _tmf882X_CMD_STAT_OK{
            //     log::info!("Command okay2");
            //     return Ok(())
            // }
            if status[1] == _tmf882X_CMD_STAT_ACCEPTED {
                log::info!("Command accepted");
                continue;
            } else if status[1] == _tmf882X_CMD_STAT_OK {
                log::info!("Command okay2");
                return Ok(());
            } else {
                log::info!("Status: {:?}", status);
            }
            // self.delay.delay_ms(1).await;
        }
        Err(Error::NoAck)
    }
    // fn read_bytes(&mut self, reg: u8, bytes: usize){
    //     let mut res = vec![0, bytes];
    //     // self.i2c
    //     self.i2c.write_read(self.address, &[reg], &mut res);

    // }
    // fn write_bytes(&mut self, reg: u8, data: [u8]){
    //     self.i2c.write(self.address, reg+ data.)
    // }
    // fn read_byte(&self, mut i2c: &I2c<'_, I2C0, Async>, reg_address: u8) {
    //
    //     i2c.blocking_write_read(self.address, &[reg_address], &mut result);
    // }
}
