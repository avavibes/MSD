use crate::sensors::core::Sensors;
#[derive(Debug)]
pub struct SensorData {
    pub viewport: Viewport,
    pub sensor: Sensors,
    pub range: Option<u32>,
    pub confidence: u32,
    pub max_range: u32,
}
#[derive(Debug)]
pub struct Viewport{
    center: (i32, i32),
    viewport: (u32, u32),
}
pub struct View {
    pub center: (i32, i32),
    pub viewport: (u32, u32),
    pub divisions: (u8, u8)
}
impl View {
    pub fn get_subdivision(&self, x: u8, y: u8) -> Viewport{
        let top_left = (self.center.0-(self.viewport.0 as i32/2), self.center.1+(self.viewport.1 as i32/2));
        let x_sw = self.viewport.0/(self.divisions.0 as u32);
        let y_sw = self.viewport.1/(self.divisions.1 as u32);
        let top_left_of_subdivision = (top_left.0+(x as i32)*(x_sw as i32), top_left.1-(y as i32)*(y_sw as i32));
        let center_of_subdivision = (top_left_of_subdivision.0+(x_sw as i32),  top_left_of_subdivision.1-(x_sw as i32));
        Viewport{ center: center_of_subdivision, viewport: (x_sw, y_sw) }
    }
}
