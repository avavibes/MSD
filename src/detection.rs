use crate::sensors::core::{SensorType, utils::Point3D};

pub struct DetectionArea {
    pub area: Area3D,
    pub sensor: [Option<SensorType>; 6],
    pub group: i32,
    pub use_assumed: bool,
}
impl DetectionArea {
    // pub fn point_severities()
}
pub struct Area3D {
    pub range_x: (i32, i32),
    pub range_y: (i32, i32),
    pub range_z: (i32, i32),
}

impl Area3D {
    pub fn contains_point(&self, p: Point3D) -> Option<i32> {
        if !(p.x < self.range_x.1 && p.x > self.range_x.0) {
            return None;
        }
        if !(p.y < self.range_y.1 && p.y > self.range_y.0) {
            return None;
        }
        if !(p.z < self.range_z.1 && p.z > self.range_z.0) {
            return None;
        }
        Some(p.z)
    }
}
