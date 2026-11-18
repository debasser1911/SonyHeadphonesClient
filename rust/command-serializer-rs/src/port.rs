#![forbid(unsafe_code)]

use byte_magic_rs::port::int_to_bytes_be;
use constants_rs::port::*;
use exceptions_rs::port::HeadphonesError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtMessage {
    pub data_type: DataType,
    pub seq_number: u8,
    pub message_bytes: Vec<u8>,
}

pub fn escape_specials(src: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(src.len());
    for &b in src {
        match b {
            60 => ret.extend([ESCAPED_BYTE_SENTRY, ESCAPED_60]),
            61 => ret.extend([ESCAPED_BYTE_SENTRY, ESCAPED_61]),
            62 => ret.extend([ESCAPED_BYTE_SENTRY, ESCAPED_62]),
            _ => ret.push(b),
        }
    }
    ret
}

pub fn unescape_specials(src: &[u8]) -> Result<Vec<u8>, HeadphonesError> {
    let mut ret = Vec::with_capacity(src.len());
    let mut it = src.iter().copied();
    while let Some(b) = it.next() {
        if b == ESCAPED_BYTE_SENTRY {
            let dec = match it.next() {
                Some(ESCAPED_60) => 60,
                Some(ESCAPED_61) => 61,
                Some(ESCAPED_62) => 62,
                None => {
                    return Err(HeadphonesError::runtime(
                        "No data left for escaped byte data",
                    ));
                }
                Some(_) => return Err(HeadphonesError::runtime("Unexpected escaped byte")),
            };
            ret.push(dec);
        } else {
            ret.push(b);
        }
    }
    Ok(ret)
}

pub fn sum_checksum(src: &[u8]) -> u8 {
    src.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

pub fn package_data_for_bt(
    src: &[u8],
    dtype: DataType,
    seq: u8,
) -> Result<Vec<u8>, HeadphonesError> {
    let mut raw = vec![dtype as u8, seq];
    raw.extend(int_to_bytes_be(src.len() as u32));
    raw.extend_from_slice(src);
    raw.push(sum_checksum(&raw));
    let mut ret = vec![START_MARKER];
    ret.extend(escape_specials(&raw));
    ret.push(END_MARKER);
    if ret.len() > MAX_BLUETOOTH_MESSAGE_SIZE {
        return Err(HeadphonesError::runtime(
            "Exceeded max bluetooth message size",
        ));
    }
    Ok(ret)
}

pub fn unpack_bt_message(src: &[u8]) -> Result<BtMessage, HeadphonesError> {
    let raw = unescape_specials(src)?;
    if raw.len() < 7 {
        return Err(HeadphonesError::runtime(
            "Invalid message: Smaller than minimum size",
        ));
    }
    let last = raw.len() - 1;
    if raw[last] != sum_checksum(&raw[..last]) {
        return Err(HeadphonesError::recoverable("Invalid checksum!", true));
    }
    let len = u32::from_be_bytes([raw[2], raw[3], raw[4], raw[5]]) as usize;
    if 6 + len > last {
        return Err(HeadphonesError::recoverable("Invalid message length", true));
    }
    Ok(BtMessage {
        data_type: DataType::from(raw[0]),
        seq_number: raw[1],
        message_bytes: raw[6..6 + len].to_vec(),
    })
}

pub fn get_dual_single_for_asm_level_with_max(
    level: i8,
    max_steps: i8,
) -> Result<NcDualSingleValue, HeadphonesError> {
    if level > max_steps {
        return Err(HeadphonesError::runtime("Exceeded max steps"));
    }
    Ok(match level {
        1 => NcDualSingleValue::Single,
        0 => NcDualSingleValue::Dual,
        _ => NcDualSingleValue::Off,
    })
}

pub fn get_dual_single_for_asm_level(level: i8) -> Result<NcDualSingleValue, HeadphonesError> {
    get_dual_single_for_asm_level_with_max(level, MAX_STEPS_WH_1000_XM3)
}

pub fn serialize_nc_and_asm_setting(
    effect: NcAsmEffect,
    nc_type: NcAsmSettingType,
    asm_type: AsmSettingType,
    asm_id: AsmId,
    level: i8,
    max_steps: i8,
) -> Result<Vec<u8>, HeadphonesError> {
    let dual = get_dual_single_for_asm_level_with_max(level, max_steps)?;
    Ok(vec![
        CommandType::NcAsmSetParam as u8,
        NcAsmInquiredType::NoiseCancellingAndAmbientSoundMode as u8,
        effect as u8,
        nc_type as u8,
        dual as u8,
        asm_type as u8,
        asm_id as u8,
        level as u8,
    ])
}

pub fn serialize_vpt_setting(t: VptInquiredType, preset: u8) -> Vec<u8> {
    vec![CommandType::VptSetParam as u8, t as u8, preset]
}
pub fn serialize_xm4_optimize_command(s: OptimizerState) -> Vec<u8> {
    vec![CommandType::Xm4OptimizerParam as u8, 0x01, 0x00, s as u8]
}
pub fn serialize_xm4_speak_to_chat(s: S2cToggle) -> Vec<u8> {
    vec![CommandType::Xm4S2cToggleParam as u8, 0x05, 0x01, s as u8]
}
pub fn serialize_xm4_s2c_options(sens: u8, voice: u8, off: u8) -> Vec<u8> {
    vec![
        CommandType::Xm4S2cOptionsParam as u8,
        0x05,
        0x00,
        sens,
        voice,
        off,
    ]
}
pub fn serialize_multi_point_command(cmd: MultiPointCommand, mac: &str) -> Vec<u8> {
    [CommandType::MultiPointParam as u8, 0x01, 0x00, cmd as u8]
        .into_iter()
        .chain(mac.bytes())
        .collect()
}
