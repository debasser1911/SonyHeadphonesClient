use command_serializer_rs::port::*;
use constants_rs::port::*;

#[test]
fn escape_and_unescape_round_trip() {
    let input = vec![1, 60, 61, 62, 255];
    let escaped = escape_specials(&input);
    assert_eq!(escaped, vec![1, 61, 44, 61, 45, 61, 46, 255]);
    assert_eq!(unescape_specials(&escaped).unwrap(), input);
    assert!(unescape_specials(&[61]).is_err());
    assert!(unescape_specials(&[61, 99]).is_err());
}

#[test]
fn package_and_unpack_handles_escaped_bytes_bug_fix() {
    // Payload containing 0x3C (60), 0x3D (61), 0x3E (62) triggers escaping
    let payload = vec![0x3C, 0x01, 0x3D, 0x3E, 0xAA];
    let framed = package_data_for_bt(&payload, DataType::DataMdrNo2, 1).unwrap();
    assert_eq!(framed[0], START_MARKER);
    assert_eq!(framed[framed.len() - 1], END_MARKER);

    let inner = &framed[1..framed.len() - 1];
    let msg = unpack_bt_message(inner).unwrap();
    assert_eq!(msg.data_type, DataType::DataMdrNo2);
    assert_eq!(msg.seq_number, 1);
    assert_eq!(msg.message_bytes, payload);
}

#[test]
fn nc_asm_xm3_vs_xm4_steps_and_xm4_commands() {
    // Step 20 fails on XM3 (max 19) and succeeds on XM4 (max 20)
    assert!(
        serialize_nc_and_asm_setting(
            NcAsmEffect::AdjustmentCompletion,
            NcAsmSettingType::LevelAdjustment,
            AsmSettingType::LevelAdjustment,
            AsmId::Normal,
            20,
            MAX_STEPS_WH_1000_XM3,
        )
        .is_err()
    );
    let xm4_step20 = serialize_nc_and_asm_setting(
        NcAsmEffect::AdjustmentCompletion,
        NcAsmSettingType::LevelAdjustment,
        AsmSettingType::LevelAdjustment,
        AsmId::Voice,
        20,
        MAX_STEPS_WH_1000_XM4,
    )
    .unwrap();
    assert_eq!(xm4_step20, vec![104, 2, 17, 1, 0, 1, 1, 20]);

    // Off mode (-1)
    let off = serialize_nc_and_asm_setting(
        NcAsmEffect::Off,
        NcAsmSettingType::LevelAdjustment,
        AsmSettingType::LevelAdjustment,
        AsmId::Normal,
        ASM_LEVEL_DISABLED,
        MAX_STEPS_WH_1000_XM4,
    )
    .unwrap();
    assert_eq!(off, vec![104, 2, 0, 1, 0, 1, 0, 0xFF]);

    assert_eq!(
        serialize_xm4_optimize_command(OptimizerState::Optimizing),
        vec![0x84, 0x01, 0x00, 0x01]
    );
    assert_eq!(
        serialize_xm4_speak_to_chat(S2cToggle::Active),
        vec![0xF8, 0x05, 0x01, 0x01]
    );
    assert_eq!(
        serialize_xm4_s2c_options(1, 1, 2),
        vec![0xFC, 0x05, 0x00, 1, 1, 2]
    );
    let mp = serialize_multi_point_command(MultiPointCommand::Connect, "11:22:33:44:55:66");
    assert_eq!(&mp[..4], &[0x3C, 0x01, 0x00, 0x01]);
    assert_eq!(&mp[4..], b"11:22:33:44:55:66");
}
