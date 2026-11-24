pub const ENABLE_TMF8821: bool = true;
pub const ENABLE_I2S: bool = false;
pub const ENABLE_HCSR04: bool = false;
pub const ENABLE_USB_LOGGING: bool = true;
pub const ENABLE_TEST_FPSIN: bool = false;
pub const LOOP_SPAD_MAPS: bool = false;

pub const ENABLE_START_WAIT: bool = true;

pub const USB_LOG_SIZE: usize = 1024;

pub const I2C0_FREQ: u32 = 400000;
pub const I2C1_FREQ: u32 = 400000;

// To avoid using the floating point unit,
// we will be using 32 bit fixed arithmetic for
// angles.
// This number denotes the n value of the fixed
// point value. (m.n), so as we are using 32 bit
// values, they will retain the necessary precision.
//
pub const DEGREE_SHIFT: i32 = 16;

// device height above the ground in mm
pub const DEVICE_HEIGHT: i32 = 1000;
