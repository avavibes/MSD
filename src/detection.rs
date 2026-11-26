use crate::sensors::core::{SensorType, utils::Point3D};

pub struct DetectionArea {
    pub area: Area3D,
    pub sensor: [Option<SensorType>; 6],
    pub group: i32,
    pub use_assumed: bool,
}
#[derive(Debug)]
pub struct DetectionResult {
    pub range: i32,
    pub max_range: i32,
}
impl DetectionArea {
    pub fn severity_from_points(&self, points: &[Option<Point3D>]) -> Option<DetectionResult> {
        let mut max_detection: Option<(i32, i32)> = None;
        for p in points.iter().flatten() {
            if p.is_confirmed || (self.use_assumed) {
                let det = self.area.contains_point(p);
                if let Some(d) = det {
                    log::info!("x{}, y{}, z{}", p.x, p.y, p.z);
                    if let Some(md) = max_detection {
                        if d.0 < md.0 {
                            max_detection = Some(d);
                        }
                    } else {
                        max_detection = Some(d);
                    }
                }
            }
        }
        max_detection.map(|md| DetectionResult {
            range: md.0,
            max_range: md.1,
        })
    }
    // pub fn severity_from_sensordatas(&self, sd: &[Option<SensorData>]) -> Option<(i32, i32)> {
    //     let mut max_detection: Option<(i32, i32)> = None;
    //     for s in sd.iter().flatten() {
    //         let p = s.to_point();
    //         // log::info!("Point from 3d data: {p:?}");
    //         if p.is_confirmed || (self.use_assumed) {
    //             let det = self.area.contains_point(p);
    //             if let Some(d) = det {
    //                 if let Some(md) = max_detection {
    //                     if d.0 < md.0 {
    //                         max_detection = Some(d);
    //                     }
    //                 } else {
    //                     max_detection = Some(d);
    //                 }
    //             }
    //         }
    //     }
    //     max_detection
    // }
    // pub fn point_severities()
}
pub struct Area3D {
    pub range_x: (i32, i32),
    pub range_y: (i32, i32),
    pub range_z: (i32, i32),
}

impl Area3D {
    pub fn contains_point(&self, p: &Point3D) -> Option<(i32, i32)> {
        if !(p.x < self.range_x.1 && p.x > self.range_x.0) {
            return None;
        }
        if !(p.y < self.range_y.1 && p.y > self.range_y.0) {
            return None;
        }
        if !(p.z < self.range_z.1 && p.z > self.range_z.0) {
            return None;
        }
        Some((p.z, self.range_z.1))
    }
}
