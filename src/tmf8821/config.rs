pub enum SPAD {
    Map3x3Normal = 1,
    Map3x3MacroUpper = 2,
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

const _TMF8821_DEFAULT_CONFIGS: [u8; 24] = [
    33, // PERIOD_MS_LSB (at address 0x24)
    0,  // PERIOD_MS_MSB (at address 0x25)
    25, // KILO_ITERATIONS_LSB (at address 0x26)
    2,  // KILO_ITERATIONS_MSB (at address 0x27)
    0,  // INT_THRESHOLD_LOW_LSB (at address 0x28)
    0,  // INT_THRESHOLD_LOW_MSB (at address 0x29)
    0,  // INT_THRESHOLD_HIGH_LSB (at address 0x2A)
    0,  // INT_THRESHOLD_HIGH_MSB (at address 0x2B)
    0,  // INT_ZONE_MASK_0 (at address 0x2C)
    0,  // INT_ZONE_MASK_1 (at address 0x2D)
    0,  // INT_ZONE_MASK_2 (at address 0x2E)
    0,  // INT_PERSISTENCE (at address 0x2F)
    6,  // CONFIDENCE_THRESHOLD (at address 0x30)
    0,  // GPIO_0 (at address 0x31)
    0,  // GPIO_1 (at address 0x32)
    0,  // POWER_CFG (at address 0x33)
    1,  // SPAD_MAP_ID (at address 0x34)
    4,  // ALG_SETTING_0 (at address 0x35)
    0,  // HIST_DUMP (at address 0x39)
    0,  // SPREAD_SPECTRUM (at address 0x3A)
    82, // I2C_SLAVE_ADDRESS (at address 0x3B) --> 0x41 << 1
    0,  // OSC_TRIM_VALUE_LSB (at address 0x3C)
    0,  // OSC_TRIM_VALUE_MSB (at address 0x3D)
    0,  // I2C_ADDR_CHANGE (at address 0x3E)
];
pub struct TmfConfiguration {
    pub config: [u8; 24],
}
impl TmfConfiguration {
    pub fn new_default() -> Self {
        TmfConfiguration {
            config: _TMF8821_DEFAULT_CONFIGS,
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
        self.config[16] = sp as u8;
    }
}
