pub mod utils;
#[derive(Debug, Clone, Copy)]
pub enum SensorType {
    TMF8821,
}

pub struct Sensor {
    pub sensor: SensorType,
    pub mount_position: (i32, i32), // Offset from the center point of the device in mm
    pub mount_angle: (i32, i32),
    pub enabled: bool,
    pub error: bool,
}
