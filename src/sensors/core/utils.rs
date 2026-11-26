use embassy_time::Timer;

use crate::{
    config::DEVICE_HEIGHT,
    core::{
        Viewport,
        math::{DEGREE_SHIFT, FixedPoint},
    },
    sensors::core::{Sensor, SensorType},
};
#[derive(Debug, Clone, Copy)]
pub struct SensorData {
    pub viewport: Viewport,
    pub sensor: SensorType,
    pub range: Option<u32>,
    pub confidence: u32,
    pub max_range: u32,
}
impl SensorData {
    pub async fn print(&self) {
        // let is_max;
        let range = match self.range {
            Some(s) => {
                // is_max = false;
                s
            }
            None => {
                // is_max = true;
                self.max_range
            }
        };
        log::info!("range {range}");

        Timer::after_millis(1).await;
        log::info!(
            "theta {} {}",
            self.viewport.center.1.raw(),
            f64::from(self.viewport.center.1),
        );

        Timer::after_millis(1).await;
        log::info!(
            "azimuth {} {}",
            self.viewport.center.0.raw(),
            f64::from(self.viewport.center.0),
        );
        Timer::after_millis(1).await;

        log::info!(
            "{} - {}",
            range as i64,
            f64::from(fpcos(self.viewport.center.1))
        );
        log::info!(
            "{} - {}",
            range as i64,
            f64::from(fpsin(self.viewport.center.1))
        );
        log::info!(
            "{} - {}",
            range as i64,
            f64::from(fpcos(self.viewport.center.0))
        );
        log::info!(
            "{} - {}",
            range as i64,
            f64::from(fpsin(self.viewport.center.0))
        );
    }
    pub fn to_point(&self, sens: &Sensor) -> Point3D {
        let is_max;
        let range = match self.range {
            Some(s) => {
                is_max = false;
                s
            }
            None => {
                is_max = true;
                self.max_range
            }
        };
        // log::info!("range {range}");
        // log::info!("theta {}", self.viewport.center.1);
        // log::info!("azimuth {}", self.viewport.center.0);
        // panic_wait("STOPPING");

        // coordinate system is spherical with the origin being directly ahead of the device.
        // viewport.center.0 is from -180 to +180 on the horizontal plane of the device
        // viewport.center.1 is from -90 to +90 on the vertical plane of the device
        // z coordinate is away from device, x coordinate is left/right, and y is up/down

        // log::info!("init theta: {}, azimuth: {}", fixed_to_float(self.viewport.center.1), fixed_to_float(self.viewport.center.0));

        let theta = sens.mount_angle.1 + self.viewport.center.1 + (90);
        let azimuth = sens.mount_angle.0 + self.viewport.center.0 + (90);
        let sint = fpsin(theta);
        let cost = fpcos(theta);
        let sina = fpsin(azimuth);
        let cosa = fpcos(azimuth);
        // let rsint = multiply_unfixed_with_fixed(range as i64, fpsin(theta));
        // log::info!("theta: {}, azimuth: {}", fixed_to_float(theta), fixed_to_float(azimuth));
        // log::info!("sint {} cost {} sina {} cosa {}", fixed_to_float(sint as i32), fixed_to_float(cost as i32), fixed_to_float(sina as i32), fixed_to_float(cosa as i32));
        // log::info!("rsint: {rsint}");
        // log::info!("sintsina: {}", fixed_to_float(multiply_fixed(sint, sina) as i32));
        let y: i32 = (cost * (range as i32)).into();
        let y = y * -1 + DEVICE_HEIGHT;
        let z = multiply_unfixed_with_fixed(range as i64, (sint * sina).raw());
        let x: i32 = -multiply_unfixed_with_fixed(range as i64, (sint * cosa).raw());

        Point3D {
            is_confirmed: !is_max,
            x,
            y,
            z,
            sensortype: self.sensor,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point3D {
    pub is_confirmed: bool,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub sensortype: SensorType,
}

pub struct View {
    pub center: (FixedPoint, FixedPoint),
    pub viewport: (FixedPoint, FixedPoint),
    pub divisions: (u8, u8),
}
impl View {
    pub fn get_subdivision(&self, x: u8, y: u8) -> Viewport {
        let top_left = (
            self.center.0 - (self.viewport.0 / 2),
            self.center.1 + (self.viewport.1 / 2),
        );
        // let bottom_right = (
        //     self.center.0 + (self.viewport.0 as i32 / 2),
        //     self.center.1 - (self.viewport.1 as i32 / 2),
        // );
        // log::info!("Top left: {:?}, bottom right: {:?}", (fixed_to_float(top_left.0), fixed_to_float(top_left.1)), (fixed_to_float(bottom_right.0), fixed_to_float(bottom_right.1)));
        let x_sw = self.viewport.0 / (self.divisions.0 as i32);
        let y_sw = self.viewport.1 / (self.divisions.1 as i32);
        let top_left_of_subdivision = (
            top_left.0 + x_sw * (x as i32),
            top_left.1 - y_sw * (y as i32),
        );
        // let bottom_right_of_subdivision = (
        //     top_left_of_subdivision.0 + (x_sw as i32),
        //     top_left_of_subdivision.1 - (y_sw as i32),
        // );
        // log::info!(
        //     "{} {} topleft {} {}",
        //     x,
        //     y,
        //     top_left_of_subdivision.0,
        //     top_left_of_subdivision.1
        // );
        // log::info!("({x}, {y}) Top left sub: {:?}, bottom right: {:?}", (fixed_to_float(top_left_of_subdivision.0), fixed_to_float(top_left_of_subdivision.1)), (fixed_to_float(bottom_right_of_subdivision.0), fixed_to_float(bottom_right_of_subdivision.1)));

        let center_of_subdivision = (
            top_left_of_subdivision.0 + (x_sw / 2),
            top_left_of_subdivision.1 - (x_sw / 2),
        );
        Viewport {
            center: center_of_subdivision,
            viewport: (x_sw, y_sw),
        }
    }
}

const LUT: [i32; 181] = [
    0, 1143, 2287, 3429, 4571, 5711, 6850, 7986, 9120, 10252, 11380, 12504, 13625, 14742, 15854,
    16961, 18064, 19160, 20251, 21336, 22414, 23486, 24550, 25606, 26655, 27696, 28729, 29752,
    30767, 31772, 32767, 33753, 34728, 35693, 36647, 37589, 38521, 39440, 40347, 41243, 42125,
    42995, 43852, 44695, 45525, 46340, 47142, 47929, 48702, 49460, 50203, 50931, 51643, 52339,
    53019, 53683, 54331, 54963, 55577, 56175, 56755, 57319, 57864, 58393, 58903, 59395, 59870,
    60326, 60763, 61183, 61583, 61965, 62328, 62672, 62997, 63302, 63589, 63856, 64103, 64331,
    64540, 64729, 64898, 65047, 65176, 65286, 65376, 65446, 65496, 65526, 65536, 65526, 65496,
    65446, 65376, 65286, 65176, 65047, 64898, 64729, 64540, 64331, 64103, 63856, 63589, 63302,
    62997, 62672, 62328, 61965, 61583, 61183, 60763, 60326, 59870, 59395, 58903, 58393, 57864,
    57319, 56755, 56175, 55577, 54963, 54331, 53683, 53019, 52339, 51643, 50931, 50203, 49460,
    48702, 47929, 47142, 46340, 45525, 44695, 43852, 42995, 42125, 41243, 40347, 39440, 38521,
    37589, 36647, 35693, 34728, 33753, 32767, 31772, 30767, 29752, 28729, 27696, 26655, 25606,
    24550, 23486, 22414, 21336, 20251, 19160, 18064, 16961, 15854, 14742, 13625, 12504, 11380,
    10252, 9120, 7986, 6850, 5711, 4571, 3429, 2287, 1143, 0,
];
pub fn fpsin(num: FixedPoint) -> FixedPoint {
    let num = num + FixedPoint::from(360);
    let num = num % (FixedPoint::from(360));
    let k: i32 = num.into();
    // let k = num >> DEGREE_SHIFT;
    let sign: i32 = if num > FixedPoint::from(180) { -1 } else { 1 };
    let k = k % 180;
    // log::info!("angle in: {}", fixed_to_float(num));
    // log::info!("Index: {k}");

    let y1 = FixedPoint::raw_from(LUT[k as usize]);
    return y1 * sign;
    // if num % (1 << DEGREE_SHIFT) == 0 {
    //     log::info!("Returning early because {}%{}==0", num, (1<<DEGREE_SHIFT));
    //     return y1 * sign;
    // }
    // let y2 = LUT[(k + 1) as usize];
    // let x1 = 0;
    // let x2 = 1 << DEGREE_SHIFT;
    // log::info!("Y1 {}, Y2 {}, X1 {x1}, X2 {x2}", fixed_to_float(y1),fixed_to_float(y2));
    // let x = num % (1 << DEGREE_SHIFT);
    // let y = (y1 * (x2 - x) + y2 * (x - x1)) / (x2 - x1);
    // log::info!("X {x}, y {}", fixed_to_float(y));
    // y * sign
}

pub fn fpcos(num: FixedPoint) -> FixedPoint {
    const OFFSET: FixedPoint = FixedPoint {
        num: 90 << DEGREE_SHIFT,
    };
    fpsin(num + OFFSET)
}

pub fn fixed_to_float(num: i32) -> f64 {
    num as f64 / (1 << DEGREE_SHIFT) as f64
}
pub fn float_to_fixed(num: f64) -> i32 {
    (num * ((1 << DEGREE_SHIFT) as f64)) as i32
}
pub fn multiply_unfixed_with_fixed(unfixed: i64, fixed: i32) -> i32 {
    (((unfixed) * (fixed as i64)) >> (DEGREE_SHIFT as i64)) as i32
}
// pub fn/
// pub fn multiply_unfixed_with_fixed
pub fn u32_to_fixed(num: u32) -> i32 {
    (num as i32) << (DEGREE_SHIFT)
}

pub fn multiply_fixed(a: i64, b: i64) -> i64 {
    (a * b) >> (DEGREE_SHIFT)
}
