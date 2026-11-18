#![forbid(unsafe_code)]

pub const MAX_BLUETOOTH_MESSAGE_SIZE: usize = 2048;
pub const START_MARKER: u8 = 62; // '>'
pub const END_MARKER: u8 = 60; // '<'
pub const ESCAPED_BYTE_SENTRY: u8 = 61; // '='
pub const ESCAPED_60: u8 = 44; // ','
pub const ESCAPED_61: u8 = 45; // '-'
pub const ESCAPED_62: u8 = 46; // '.'
pub const MAC_ADDR_STR_SIZE: usize = 17;
pub const MINIMUM_VOICE_FOCUS_STEP: i32 = 2;
pub const ASM_LEVEL_DISABLED: i8 = -1;
pub const MAX_STEPS_WH_1000_XM3: i8 = 19;
pub const MAX_STEPS_WH_1000_XM4: i8 = 20;
pub const APP_VERSION: &str = "2.0.0";
pub const SERVICE_UUID: &str = "96CC203E-5068-46ad-B32D-E316F5E069BA";
pub const SERVICE_UUID_IN_BYTES: [u8; 16] = [
    0x96, 0xcc, 0x20, 0x3e, 0x50, 0x68, 0x46, 0xad, 0xb3, 0x2d, 0xe3, 0x16, 0xf5, 0xe0, 0x69, 0xba,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum DataType {
    Data = 0,
    Ack = 1,
    DataMcNo1 = 2,
    DataIcd = 9,
    DataEv = 10,
    DataMdr = 12,
    DataCommon = 13,
    DataMdrNo2 = 14,
    Shot = 16,
    ShotMcNo1 = 18,
    ShotIcd = 25,
    ShotEv = 26,
    ShotMdr = 28,
    ShotCommon = 29,
    ShotMdrNo2 = 30,
    LargeDataCommon = 45,
    Unknown = -1,
}

impl From<u8> for DataType {
    fn from(v: u8) -> Self {
        match v as i8 {
            0 => Self::Data,
            1 => Self::Ack,
            2 => Self::DataMcNo1,
            9 => Self::DataIcd,
            10 => Self::DataEv,
            12 => Self::DataMdr,
            13 => Self::DataCommon,
            14 => Self::DataMdrNo2,
            16 => Self::Shot,
            18 => Self::ShotMcNo1,
            25 => Self::ShotIcd,
            26 => Self::ShotEv,
            28 => Self::ShotMdr,
            29 => Self::ShotCommon,
            30 => Self::ShotMdrNo2,
            45 => Self::LargeDataCommon,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcAsmInquiredType {
    NoUse = 0,
    NoiseCancelling = 1,
    NoiseCancellingAndAmbientSoundMode = 2,
    AmbientSoundMode = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcAsmEffect {
    Off = 0,
    On = 1,
    AdjustmentInProgress = 16,
    AdjustmentCompletion = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcAsmSettingType {
    OnOff = 0,
    LevelAdjustment = 1,
    DualSingleOff = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AsmSettingType {
    OnOff = 0,
    LevelAdjustment = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AsmId {
    Normal = 0,
    Voice = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcDualSingleValue {
    Off = 0,
    Single = 1,
    Dual = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandType {
    InitRequest = 0x00,
    InitResponse = 0x01,
    MacQuery = 0x02,
    CapabilityQuery = 0x06,
    CapabilityQueryResponse = 0x07,
    MultiPointDevicesQuery = 0x36,
    DevicesQueryResponse = 0x37,
    DevicesStateResponse = 0x39,
    MultiPointParam = 0x3C,
    EqGet = 0x56,
    EqRet = 0x57,
    EqSet = 0x58,
    EqNotify = 0x59,
    VptSetParam = 72,
    NcAsmQuery = 0x66,
    NcAsmRetParam = 0x67,
    NcAsmSetParam = 104,
    NcAsmNotify = 0x69,
    Xm4OptimizerParam = 0x84,
    Xm4OptimizerResponse = 0x85,
    DseeGet = 0xE6,
    DseeRet = 0xE7,
    DseeSet = 0xE8,
    DseeNotify = 0xE9,
    S2cQuery = 0xF6,
    S2cQueryResponse = 0xF7,
    Xm4S2cToggleParam = 0xF8,
    S2cNotify = 0xF9,
    S2cOptionsQuery = 0xFA,
    S2cOptionsResponse = 0xFB,
    Xm4S2cOptionsParam = 0xFC,
    S2cOptionsNotify = 0xFD,
}

impl TryFrom<u8> for CommandType {
    type Error = u8;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::InitRequest),
            0x01 => Ok(Self::InitResponse),
            0x02 => Ok(Self::MacQuery),
            0x06 => Ok(Self::CapabilityQuery),
            0x07 => Ok(Self::CapabilityQueryResponse),
            0x36 => Ok(Self::MultiPointDevicesQuery),
            0x37 => Ok(Self::DevicesQueryResponse),
            0x39 => Ok(Self::DevicesStateResponse),
            0x3C => Ok(Self::MultiPointParam),
            0x56 => Ok(Self::EqGet),
            0x57 => Ok(Self::EqRet),
            0x58 => Ok(Self::EqSet),
            0x59 => Ok(Self::EqNotify),
            72 => Ok(Self::VptSetParam),
            0x66 => Ok(Self::NcAsmQuery),
            0x67 => Ok(Self::NcAsmRetParam),
            104 => Ok(Self::NcAsmSetParam),
            0x69 => Ok(Self::NcAsmNotify),
            0x84 => Ok(Self::Xm4OptimizerParam),
            0x85 => Ok(Self::Xm4OptimizerResponse),
            0xE6 => Ok(Self::DseeGet),
            0xE7 => Ok(Self::DseeRet),
            0xE8 => Ok(Self::DseeSet),
            0xE9 => Ok(Self::DseeNotify),
            0xF6 => Ok(Self::S2cQuery),
            0xF7 => Ok(Self::S2cQueryResponse),
            0xF8 => Ok(Self::Xm4S2cToggleParam),
            0xF9 => Ok(Self::S2cNotify),
            0xFA => Ok(Self::S2cOptionsQuery),
            0xFB => Ok(Self::S2cOptionsResponse),
            0xFC => Ok(Self::Xm4S2cOptionsParam),
            0xFD => Ok(Self::S2cOptionsNotify),
            other => Err(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VptPresetId {
    Off = 0,
    OutdoorFestival = 1,
    Arena = 2,
    ConcertHall = 3,
    Club = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum SoundPositionPreset {
    Off = 0,
    FrontLeft = 1,
    FrontRight = 2,
    Front = 3,
    RearLeft = 17,
    RearRight = 18,
    OutOfRange = -1,
}

pub const SOUND_POSITION_PRESET_ARRAY: [SoundPositionPreset; 7] = [
    SoundPositionPreset::Off,
    SoundPositionPreset::FrontLeft,
    SoundPositionPreset::FrontRight,
    SoundPositionPreset::Front,
    SoundPositionPreset::RearLeft,
    SoundPositionPreset::RearRight,
    SoundPositionPreset::OutOfRange,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum VptInquiredType {
    NoUse = 0,
    Vpt = 1,
    SoundPosition = 2,
    OutOfRange = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OptimizerState {
    Idle = 0,
    Optimizing = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum S2cToggle {
    Inactive = 0,
    Active = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum S2cSensitivity {
    Auto = 0,
    High = 1,
    Low = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum S2cAutoOff {
    Short = 0,
    Standard = 1,
    Long = 2,
    Off = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MultiPointCommand {
    Disconnect = 0x00,
    Connect = 0x01,
    Unpair = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum FunctionType {
    DeviceManagement = 56,
    Vpt = 65,
    Optimizer = -127,
    NcAsm = 98,
    SmartTalkingMode = -11,
}

impl FunctionType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b as i8 {
            56 => Some(Self::DeviceManagement),
            65 => Some(Self::Vpt),
            -127 => Some(Self::Optimizer),
            98 => Some(Self::NcAsm),
            -11 => Some(Self::SmartTalkingMode),
            _ => None,
        }
    }
}

pub mod device_capabilities {
    pub const NC_ASM: u32 = 1 << 0;
    pub const VPT: u32 = 1 << 1;
    pub const MULTI_POINT: u32 = 1 << 2;
    pub const OPTIMIZER: u32 = 1 << 3;
    pub const SPEAK_TO_CHAT: u32 = 1 << 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadphoneModel {
    Wh1000Xm3,
    Wh1000Xm4,
    Generic,
}

impl HeadphoneModel {
    pub fn from_device_name(name: &str) -> Self {
        let up = name.to_ascii_uppercase();
        if up.contains("XM4") {
            Self::Wh1000Xm4
        } else if up.contains("XM3") {
            Self::Wh1000Xm3
        } else {
            Self::Generic
        }
    }

    pub fn max_asm_steps(self) -> i8 {
        match self {
            Self::Wh1000Xm4 => MAX_STEPS_WH_1000_XM4,
            Self::Wh1000Xm3 | Self::Generic => MAX_STEPS_WH_1000_XM3,
        }
    }

    pub fn default_capabilities(self) -> u32 {
        use device_capabilities::*;
        match self {
            Self::Wh1000Xm4 => NC_ASM | MULTI_POINT | OPTIMIZER | SPEAK_TO_CHAT,
            Self::Wh1000Xm3 | Self::Generic => NC_ASM | VPT,
        }
    }
}
