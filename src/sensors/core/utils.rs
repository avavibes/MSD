use crate::{config::DEGREE_SHIFT, core::Viewport, sensors::core::SensorType};
#[derive(Debug)]
pub struct SensorData {
    pub viewport: Viewport,
    pub sensor: SensorType,
    pub range: Option<u32>,
    pub confidence: u32,
    pub max_range: u32,
}

pub struct View {
    pub center: (i32, i32),
    pub viewport: (u32, u32),
    pub divisions: (u8, u8),
}
impl View {
    pub fn get_subdivision(&self, x: u8, y: u8) -> Viewport {
        let top_left = (
            self.center.0 - (self.viewport.0 as i32 / 2),
            self.center.1 + (self.viewport.1 as i32 / 2),
        );
        let x_sw = self.viewport.0 / (self.divisions.0 as u32);
        let y_sw = self.viewport.1 / (self.divisions.1 as u32);
        let top_left_of_subdivision = (
            top_left.0 + (x as i32) * (x_sw as i32),
            top_left.1 - (y as i32) * (y_sw as i32),
        );
        log::info!(
            "{} {} topleft {} {}",
            x,
            y,
            top_left_of_subdivision.0,
            top_left_of_subdivision.1
        );

        let center_of_subdivision = (
            top_left_of_subdivision.0 + (x_sw as i32 / 2),
            top_left_of_subdivision.1 - (x_sw as i32 / 2),
        );
        Viewport {
            center: center_of_subdivision,
            viewport: (x_sw, y_sw),
        }
    }
}

const LUT: [i32; 181] = [
    0, 18300, 36594, 54878, 73144, 91389, 109606, 127789, 145933, 164033, 182083, 200077, 218011,
    235878, 253673, 271391, 289026, 306573, 324027, 341382, 358634, 375776, 392803, 409711, 426494,
    443147, 459665, 476043, 492276, 508359, 524287, 540056, 555660, 571095, 586356, 601438, 616337,
    631048, 645567, 659890, 674011, 687927, 701634, 715127, 728402, 741455, 754282, 766879, 779243,
    791370, 803255, 814896, 826289, 837430, 848315, 858943, 869308, 879409, 889242, 898805, 908093,
    917105, 925837, 934288, 942453, 950332, 957921, 965219, 972222, 978930, 985339, 991448, 997255,
    1002758, 1007955, 1012846, 1017428, 1021701, 1025662, 1029310, 1032645, 1035666, 1038371,
    1040760, 1042831, 1044585, 1046021, 1047138, 1047937, 1048416, 1048576, 1048416, 1047937,
    1047138, 1046021, 1044585, 1042831, 1040760, 1038371, 1035666, 1032645, 1029310, 1025662,
    1021701, 1017428, 1012846, 1007955, 1002758, 997255, 991448, 985339, 978930, 972222, 965219,
    957921, 950332, 942453, 934288, 925837, 917105, 908093, 898805, 889242, 879409, 869308, 858943,
    848315, 837430, 826289, 814896, 803255, 791370, 779243, 766879, 754282, 741455, 728402, 715127,
    701634, 687927, 674011, 659890, 645567, 631048, 616337, 601438, 586356, 571095, 555660, 540056,
    524287, 508359, 492276, 476043, 459665, 443147, 426494, 409711, 392803, 375776, 358634, 341382,
    324027, 306573, 289026, 271391, 253673, 235878, 218011, 200077, 182083, 164033, 145933, 127789,
    109606, 91389, 73144, 54878, 36594, 18300, 0,
];
pub fn fpsin(num: i32) -> i32 {
    let num = num % (360 << DEGREE_SHIFT);
    let k = num >> DEGREE_SHIFT;
    let sign: i32;
    if num > 180 << DEGREE_SHIFT {
        sign = -1;
    } else {
        sign = 1;
    }
    let k = k % 180;
    // log::info!("Index: {k}");
    let y1 = LUT[k as usize];
    if num % (1 << DEGREE_SHIFT) == 0 {
        // log::info!("Returning early because {}%{}==0", num, (1<<DEGREE_SHIFT));
        return y1 * sign;
    }
    let y2 = LUT[(k + 1) as usize];
    let x1 = 0;
    let x2 = 1 << DEGREE_SHIFT;
    // log::info!("Y1 {y1}, Y2 {y2}, X1 {x1}, X2 {x2}");
    let x = num % (1 << DEGREE_SHIFT);
    let y = (y1 * (x2 - x) + y2 * (x - x1)) / (x2 - x1);
    // log::info!("X {x}, y {y}");
    y * sign
}

pub fn fpcos(num: i32) -> i32 {
    const OFFSET: i32 = 90 << DEGREE_SHIFT;
    fpsin(num + OFFSET)
}

pub fn fixed_to_float(num: i32) -> f64 {
    num as f64 / (1 << DEGREE_SHIFT) as f64
}
pub fn float_to_fixed(num: f64) -> i32 {
    (num * ((1 << DEGREE_SHIFT) as f64)) as i32
}
