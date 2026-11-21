use crate::sensors::core::Sensors;
pub mod utils;
const DEGREE_SHIFT: u8 = 12;
pub struct Sensitivity {
    pub view: Viewport,
    pub valid_sensors: [Option<Sensors>; 5],
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
#[derive(Debug)]
pub struct Viewport {
    pub center: (i32, i32),
    pub viewport: (u32, u32),
}
// impl Viewport{
//     pub fn from_degrees(center: (i32, i32), viewport: (u32, u32)) -> Self{
//         return Viewport { center: (center.0<<DEGREE_SHIFT, center.1<<DEGREE_SHIFT), viewport: (viewport.0<<DEGREE_SHIFT, viewport.1<<DEGREE_SHIFT) }
//     }
// }

pub struct Curve {}
