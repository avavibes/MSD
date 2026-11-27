//Enable the tmf8821 sensor.
pub const ENABLE_TMF8821: bool = false;

// Enables the I2S audio module.
pub const ENABLE_I2S: bool = false;

// Enable the HCSR04 Distance sensor.
pub const ENABLE_HCSR04: bool = false;
// Enables the usb logging task.
pub const ENABLE_USB_LOGGING: bool = true;

// Enables a test of the fixed point trig functions.
pub const ENABLE_TEST_FPSIN: bool = true;

// Enables a sequence of looping through the
// different SPAD maps.
pub const LOOP_SPAD_MAPS: bool = false;

// Enable a simple test of the fixed point
// number system.
pub const FIXED_POINT_TEST: bool = false;

// Enables blinking the led every 200ms, to
// give an external indicator of whether the
// device has panicked.
pub const ENABLE_LED_BLINK: bool = true;

// Resets the device after a panic.
pub const ENABLE_WATCHDOG_PANIC: bool = true;

pub const ENABLE_TEMP_SENSOR_HCSR04: bool = true;

// Enables a 3 second wait on startup to allow
// usb serial to connect before running further.
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

// device height above the ground in mm
pub const DEVICE_HEIGHT: i32 = 800;
