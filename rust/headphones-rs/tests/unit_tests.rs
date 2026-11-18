use bluetooth_connector_rs::port::{BluetoothDevice, MockBluetoothConnector};
use bluetooth_wrapper_rs::port::BluetoothWrapper;
use command_serializer_rs::port::package_data_for_bt;
use constants_rs::port::*;
use headphones_rs::port::Headphones;

#[test]
fn xm4_capability_discovery_nc_asm_20_s2c_optimizer_and_multipoint() {
    let mock = MockBluetoothConnector::new(vec![BluetoothDevice {
        name: "WH-1000XM4".into(),
        mac: "11:22:33:44:55:66".into(),
    }]);
    let sent = mock.sent_frames.clone();
    let recv = mock.recv_queue.clone();

    let wrapper = BluetoothWrapper::new(Box::new(mock));
    wrapper.connect("11:22:33:44:55:66").unwrap();

    let mut hp = Headphones::default();
    hp.configure_for_device("WH-1000XM4");
    assert_eq!(hp.model(), HeadphoneModel::Wh1000Xm4);
    assert_eq!(hp.max_asm_steps(), 20);

    // Prepare 7 ACKs + XM4 capability, multipoint device list, and S2C replies
    let cap_reply = package_data_for_bt(
        &[
            CommandType::CapabilityQueryResponse as u8,
            FunctionType::DeviceManagement as u8,
            FunctionType::Optimizer as u8,
            FunctionType::NcAsm as u8,
            FunctionType::SmartTalkingMode as u8,
        ],
        DataType::DataMdr,
        0,
    )
    .unwrap();
    let mut dev_payload = vec![CommandType::DevicesQueryResponse as u8, 0x01, 0x01];
    dev_payload.extend_from_slice(b"AA:BB:CC:DD:EE:FF");
    dev_payload.push(0x01); // connected in slot 1
    dev_payload.push(6);
    dev_payload.extend_from_slice(b"Laptop");
    let dev_reply = package_data_for_bt(&dev_payload, DataType::DataMdrNo2, 0).unwrap();

    {
        let mut q = recv.lock().unwrap();
        // Query 1..3 + capability reply + acks + devices reply
        for i in 0..3 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, i % 2).unwrap());
        }
        q.push_back(cap_reply);
        q.push_back(package_data_for_bt(&[], DataType::Ack, 1).unwrap());
        q.push_back(dev_reply);
        for _ in 0..5 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, 0).unwrap());
        }
    }

    hp.query_state(&wrapper).unwrap();
    assert_ne!(hp.capabilities() & device_capabilities::MULTI_POINT, 0);
    assert_ne!(hp.capabilities() & device_capabilities::SPEAK_TO_CHAT, 0);
    assert_eq!(hp.capabilities() & device_capabilities::VPT, 0);
    assert_eq!(hp.saved_devices.len(), 2);
    assert_eq!(hp.saved_devices[1].name, "Laptop");
    assert_eq!(hp.get_connected_device_indices(), (1, 0));

    // Exercise XM4 controls: ASM level 20 + Focus on Voice + S2C + Optimizer + Multipoint disconnect
    hp.ambient_sound_control.desired = true;
    hp.asm_level.desired = 20;
    hp.focus_on_voice.desired = true;
    hp.speak_to_chat.desired = S2cToggle::Active;
    hp.set_s2c_options(1, true, 2);
    hp.optimizer_state.desired = OptimizerState::Optimizing;
    hp.set_multi_point_connection(1, 0);
    assert!(hp.is_changed());

    {
        let mut q = recv.lock().unwrap();
        for i in 0..5 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, i % 2).unwrap());
        }
    }
    hp.set_changes(&wrapper).unwrap();
    assert!(!hp.is_changed());
    assert_eq!(hp.asm_level.current, 20);
    assert!(hp.is_focus_on_voice_available());
    assert_eq!(hp.speak_to_chat.current, S2cToggle::Active);
    assert_eq!(hp.optimizer_state.current, OptimizerState::Optimizing);
    assert_eq!(hp.get_connected_device_indices(), (0, 0));
    assert!(!sent.lock().unwrap().is_empty());
}
