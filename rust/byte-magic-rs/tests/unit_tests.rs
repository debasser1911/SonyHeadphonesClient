use byte_magic_rs::port::*;

#[test]
fn endian_and_be_bytes_match_cpp() {
    assert_eq!(byte_order_swap(0x1234_5678), 0x7856_3412);
    assert_eq!(int_to_bytes_be(0x1234_5678), vec![0x12, 0x34, 0x56, 0x78]);
    assert_eq!(int_to_bytes_be(5), vec![0, 0, 0, 5]);
}

#[test]
fn mac_round_trip_and_validation() {
    assert_eq!(
        mac_string_to_long("01:23:45:67:89:ab").unwrap(),
        0x0123_4567_89AB
    );
    assert_eq!(
        mac_string_to_long("01-23-45-67-89-AB").unwrap(),
        0x0123_4567_89AB
    );
    assert!(mac_string_to_long("short").is_err());
    assert!(mac_string_to_long("01_23_45_67_89_ab").is_err());
    assert!(mac_string_to_long("ZZ:23:45:67:89:ab").is_err());

    let bytes = [0xAB, 0x89, 0x67, 0x45, 0x23, 0x01];
    assert_eq!(mac_bytes_to_string(&bytes), "01:23:45:67:89:ab");
}
