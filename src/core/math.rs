use core::ops::{Add, Div, Mul, Rem, Sub};
pub const DEGREE_SHIFT: i32 = 16;
#[derive(Debug, Clone, Copy)]
pub struct FixedPoint {
    pub num: i32,
}
impl FixedPoint {
    pub fn raw(&self) -> i32 {
        self.num
    }
    pub fn raw_from(num: i32) -> FixedPoint {
        FixedPoint { num }
    }
    // pub fn sin(&self) -> FixedPoint{

    // }
}

// pub struct FixedPointBig {
//     num: i64,
// }

impl Add for FixedPoint {
    type Output = FixedPoint;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            num: rhs.num + self.num,
        }
    }
}
impl Add<i32> for FixedPoint {
    type Output = FixedPoint;

    fn add(self, rhs: i32) -> Self::Output {
        Self {
            num: self.num + (rhs << DEGREE_SHIFT),
        }
    }
}
impl Sub for FixedPoint {
    type Output = FixedPoint;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            num: self.num - rhs.num,
        }
    }
}
impl Sub<i32> for FixedPoint {
    type Output = FixedPoint;
    fn sub(self, rhs: i32) -> Self::Output {
        Self {
            num: self.num - rhs,
        }
    }
}
impl From<f64> for FixedPoint {
    fn from(value: f64) -> Self {
        Self {
            num: (value * ((1 << DEGREE_SHIFT) as f64)) as i32,
        }
    }
}
impl From<i32> for FixedPoint {
    fn from(value: i32) -> Self {
        Self {
            num: value << DEGREE_SHIFT,
        }
    }
}
impl From<FixedPoint> for f64 {
    fn from(value: FixedPoint) -> Self {
        value.num as f64 / (1 << DEGREE_SHIFT) as f64
    }
}
impl Mul for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: Self) -> Self::Output {
        let a: i64 = self.num as i64 * rhs.num as i64;
        FixedPoint {
            num: (a >> DEGREE_SHIFT) as i32,
        }
    }
}
impl Mul<i32> for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: i32) -> Self::Output {
        FixedPoint {
            num: self.num * rhs,
        }
    }
}
impl Rem for FixedPoint {
    type Output = FixedPoint;

    fn rem(self, rhs: Self) -> Self::Output {
        FixedPoint {
            num: self.num % rhs.num,
        }
    }
}
impl Rem<i32> for FixedPoint {
    type Output = FixedPoint;

    fn rem(self, rhs: i32) -> Self::Output {
        FixedPoint {
            num: self.num % rhs,
        }
    }
}
impl PartialEq for FixedPoint {
    fn eq(&self, other: &Self) -> bool {
        self.num == other.num
    }
}
impl PartialOrd for FixedPoint {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.num.partial_cmp(&other.num)
    }
}
impl Div for FixedPoint {
    type Output = FixedPoint;

    fn div(self, rhs: FixedPoint) -> Self::Output {
        FixedPoint {
            num: (((self.num as i64) << DEGREE_SHIFT) / rhs.num as i64) as i32,
        }
    }
}

impl Div<i32> for FixedPoint {
    type Output = FixedPoint;

    fn div(self, rhs: i32) -> Self::Output {
        FixedPoint {
            num: self.num / rhs,
        }
    }
}
// impl Into<f64> for FixedPoint {
//     fn into(self) -> f64 {
//         self.num as f64 / (1 << DEGREE_SHIFT) as f64
//     }
// }

impl Into<i32> for FixedPoint {
    fn into(self) -> i32 {
        self.num >> DEGREE_SHIFT
    }
}
