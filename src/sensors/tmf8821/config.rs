use crate::sensors::core::utils::View;

#[derive(Debug, PartialEq)]
pub enum TofActiveRange {
    LONG,
    SHORT,
}
// pub struct
pub enum Fov {
    Rectangular(u32, u32),           // width/height
    MacroRectangular(u32, u32, u32), // top, bottom, width
}
#[derive(Debug, Clone, Copy)]
pub enum SPAD {
    Map3x3Normal = 1,     // 33x32
    Map3x3MacroUpper = 2, // 30 up, 17 down, 33
    Map3x3MacroLower = 3,
    Map3x3Wide = 6,
    Map3x3Checkerboard = 11,
    Map3x3InvCheckerboard = 12,
    Map4x4Normal = 7,
    Map4x4MacroUpper = 4,
    Map4x4MacroLower = 5,
    Map4x4Narrow = 13,
    Map3x6 = 10,
}
impl SPAD {
    pub fn fov(&self) -> Fov {
        match &self {
            SPAD::Map3x3Normal => Fov::Rectangular(33 << 6, 32 << 6),
            SPAD::Map3x3MacroUpper => Fov::MacroRectangular(30 << 6, 17 << 6, 33 << 6),
            SPAD::Map3x3MacroLower => Fov::MacroRectangular(17 << 6, 30 << 6, 33 << 6),
            SPAD::Map3x3Wide => Fov::Rectangular(41 << 6, 52 << 6),
            SPAD::Map3x3Checkerboard => Fov::Rectangular(33 << 6, 32 << 6),
            SPAD::Map3x3InvCheckerboard => Fov::Rectangular(33 << 6, 32 << 6),
            SPAD::Map4x4Normal => Fov::Rectangular(41 << 6, 52 << 6),
            SPAD::Map4x4MacroUpper => Fov::MacroRectangular(30 << 6, 17 << 6, 33 << 6),
            SPAD::Map4x4MacroLower => Fov::MacroRectangular(17 << 6, 30 << 6, 33 << 6),
            SPAD::Map4x4Narrow => Fov::Rectangular(33 << 6, 42 << 6),
            SPAD::Map3x6 => Fov::Rectangular(33 << 6, 60 << 6),
        }
    }
    pub fn divisions(&self) -> (u8, u8) {
        match &self {
            SPAD::Map3x3Normal => (3, 3),
            SPAD::Map3x3MacroUpper => (3, 3),
            SPAD::Map3x3MacroLower => (3, 3),
            SPAD::Map3x3Wide => (3, 3),
            SPAD::Map3x3Checkerboard => (3, 3),
            SPAD::Map3x3InvCheckerboard => (3, 3),
            SPAD::Map4x4Normal => (4, 4),
            SPAD::Map4x4MacroUpper => (4, 4),
            SPAD::Map4x4MacroLower => (4, 4),
            SPAD::Map4x4Narrow => (4, 4),
            SPAD::Map3x6 => (3, 6),
        }
    }
    pub fn as_view(&self) -> View {
        let center: (i32, i32);
        let width: u32;
        let height: u32;
        match self.fov() {
            Fov::Rectangular(w, h) => {
                center = (0, 0);
                height = h;
                width = w;
            }
            Fov::MacroRectangular(t, b, w) => {
                center = (0, ((t as i32 - b as i32) / 2));
                height = t + b;
                width = w;
            }
        };
        View {
            center,
            viewport: (width, height),
            divisions: self.divisions(),
        }
    }
    pub fn index_as_xy(&self, index: u8) -> Option<(u8, u8)> {
        let i;
        if self.divisions().0 == 4 {
            if index == 8 || index == 17 {
                return None;
            }
            if index > 8 {
                i = index - 1;
            } else {
                i = index;
            }
        } else {
            i = index;
        }
        Some((i % self.divisions().0, i / self.divisions().0))
    }
}

const _TMF8821_DEFAULT_CONFIGS: [u8; 24] = [
    33,      // PERIOD_MS_LSB (at address 0x24) // For 30hz
    0,       // PERIOD_MS_MSB (at address 0x25)
    25,      // KILO_ITERATIONS_LSB (at address 0x26)
    2,       // KILO_ITERATIONS_MSB (at address 0x27)
    0,       // INT_THRESHOLD_LOW_LSB (at address 0x28)
    0,       // INT_THRESHOLD_LOW_MSB (at address 0x29)
    0,       // INT_THRESHOLD_HIGH_LSB (at address 0x2A)
    0,       // INT_THRESHOLD_HIGH_MSB (at address 0x2B)
    0,       // INT_ZONE_MASK_0 (at address 0x2C)
    0,       // INT_ZONE_MASK_1 (at address 0x2D)
    0,       // INT_ZONE_MASK_2 (at address 0x2E)
    0,       // INT_PERSISTENCE (at address 0x2F)
    6,       // CONFIDENCE_THRESHOLD (at address 0x30)
    0,       // GPIO_0 (at address 0x31)
    0,       // GPIO_1 (at address 0x32)
    0,       // POWER_CFG (at address 0x33)
    1,       // SPAD_MAP_ID (at address 0x34)
    4,       // ALG_SETTING_0 (at address 0x35)
    0,       // HIST_DUMP (at address 0x39)
    0,       // SPREAD_SPECTRUM (at address 0x3A)
    66 << 1, // I2C_SLAVE_ADDRESS (at address 0x3B) --> 0x41 << 1
    0,       // OSC_TRIM_VALUE_LSB (at address 0x3C)
    0,       // OSC_TRIM_VALUE_MSB (at address 0x3D)
    0,       // I2C_ADDR_CHANGE (at address 0x3E)
];
pub struct TmfConfiguration {
    pub config: [u8; 24],
    pub spad: SPAD,
}
impl TmfConfiguration {
    pub fn new_default() -> Self {
        TmfConfiguration {
            config: _TMF8821_DEFAULT_CONFIGS,
            spad: SPAD::Map3x3Normal,
        }
    }
    pub fn period_ms(&self) -> u16 {
        self.config[0] as u16 + ((self.config[1] as u16) << 8)
    }
    pub fn set_period_ms(&mut self, value: u16) {
        self.config[0] = (value % 0xFF) as u8;
        self.config[1] = (value >> 8) as u8;
    }
    pub fn set_spad_map(&mut self, sp: SPAD) {
        self.spad = sp;
        self.config[16] = sp as u8;
    }
    pub fn set_i2c_addr(&mut self, addr: u8) {
        self.config[20] = addr << 1;
        log::info!("ADDRESS: {}", self.config[20] >> 1);
    }
}
