use crate::{core::math::FixedPoint, sensors::core::SensorType};
pub mod math;
pub mod utils;

// const DEGREE_SHIFT: u8 = 12;
pub struct Sensitivity {
    pub view: Viewport,
    pub valid_sensors: [Option<SensorType>; 5],
    pub allow_partial: bool,
    pub alert_type: u32,
    pub curve: Curve,
}
pub struct Alert {
    pub group: u32,
    pub speaker_freq: u32,
    pub speaker_amplitude: [u32; 2],
    pub pulse_len: u32,
    pub haptic_strength: u32,
    pub time_pd: u32,
    pub combine: CombineMode,
}
pub enum CombineMode {}
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub center: (FixedPoint, FixedPoint),
    pub viewport: (FixedPoint, FixedPoint),
}
// impl Viewport{
//     pub fn from_degrees(center: (i32, i32), viewport: (u32, u32)) -> Self{
//         return Viewport { center: (center.0<<DEGREE_SHIFT, center.1<<DEGREE_SHIFT), viewport: (viewport.0<<DEGREE_SHIFT, viewport.1<<DEGREE_SHIFT) }
//     }
// }

pub struct Curve {}
