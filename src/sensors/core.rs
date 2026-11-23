pub mod utils;
#[derive(Debug)]
pub enum SensorType {
    TMF8821,
}

pub struct Sensor {
    pub sensor: SensorType,
    pub mount_position: (i32, i32), // Offset from the center point of the device in mm
    pub mount_angle: (i32, i32),
}
