use constants_rs::port::*;

#[test]
fn protocol_constants_and_enums_match_spec() {
    assert_eq!(MAX_BLUETOOTH_MESSAGE_SIZE, 2048);
    assert_eq!(START_MARKER, 62);
    assert_eq!(END_MARKER, 60);
    assert_eq!(DataType::from(12), DataType::DataMdr);
    assert_eq!(DataType::from(14), DataType::DataMdrNo2);
    assert_eq!(DataType::from(1), DataType::Ack);
    assert_eq!(DataType::from(99), DataType::Unknown);
    assert_eq!(
        CommandType::try_from(0xF8),
        Ok(CommandType::Xm4S2cToggleParam)
    );
    assert_eq!(
        CommandType::try_from(0x84),
        Ok(CommandType::Xm4OptimizerParam)
    );
    assert_eq!(
        CommandType::try_from(0x3C),
        Ok(CommandType::MultiPointParam)
    );
}

#[test]
fn xm4_model_detection_and_capabilities() {
    let xm4 = HeadphoneModel::from_device_name("WH-1000XM4");
    assert_eq!(xm4, HeadphoneModel::Wh1000Xm4);
    assert_eq!(xm4.max_asm_steps(), 20);
    assert_ne!(
        xm4.default_capabilities() & device_capabilities::SPEAK_TO_CHAT,
        0
    );
    assert_ne!(
        xm4.default_capabilities() & device_capabilities::OPTIMIZER,
        0
    );
    assert_ne!(
        xm4.default_capabilities() & device_capabilities::MULTI_POINT,
        0
    );
    assert_eq!(xm4.default_capabilities() & device_capabilities::VPT, 0);

    let xm3 = HeadphoneModel::from_device_name("WH-1000XM3");
    assert_eq!(xm3, HeadphoneModel::Wh1000Xm3);
    assert_eq!(xm3.max_asm_steps(), 19);
    assert_ne!(xm3.default_capabilities() & device_capabilities::VPT, 0);
}
