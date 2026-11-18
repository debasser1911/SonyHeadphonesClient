#![forbid(unsafe_code)]

use constants_rs::port::MAC_ADDR_STR_SIZE;
use exceptions_rs::port::HeadphonesError;

pub fn byte_order_swap(num: u32) -> u32 {
    num.swap_bytes()
}

pub fn int_to_bytes_be(num: u32) -> Vec<u8> {
    num.to_be_bytes().to_vec()
}

pub fn mac_string_to_long(s: &str) -> Result<u64, HeadphonesError> {
    if s.len() != MAC_ADDR_STR_SIZE {
        return Err(HeadphonesError::runtime(format!(
            "Invalid MAC address size ({s}): {} != {MAC_ADDR_STR_SIZE}",
            s.len()
        )));
    }
    let bytes = s.as_bytes();
    let mut result = 0u64;
    for i in 0..6 {
        let off = i * 3;
        if i > 0 && bytes[off - 1] != b':' && bytes[off - 1] != b'-' {
            return Err(HeadphonesError::runtime(format!(
                "Invalid MAC address format: {s}"
            )));
        }
        let hex = std::str::from_utf8(&bytes[off..off + 2])
            .map_err(|_| HeadphonesError::runtime(format!("Invalid MAC address format: {s}")))?;
        let byte = u8::from_str_radix(hex, 16)
            .map_err(|_| HeadphonesError::runtime(format!("Invalid MAC address format: {s}")))?;
        result = (result << 8) + u64::from(byte);
    }
    Ok(result)
}

pub fn mac_bytes_to_string(addr: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0]
    )
}
