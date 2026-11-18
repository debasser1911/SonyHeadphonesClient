use bluetooth_connector_rs::port::{BluetoothDevice, MockBluetoothConnector};
use bluetooth_wrapper_rs::port::BluetoothWrapper;
use command_serializer_rs::port::package_data_for_bt;
use constants_rs::port::{CommandType, DataType};

#[test]
fn sends_command_receives_interleaved_data_and_acks() {
    let mock = MockBluetoothConnector::new(vec![BluetoothDevice {
        name: "WH-1000XM4".into(),
        mac: "11:22:33:44:55:66".into(),
    }]);
    let sent_ref = mock.sent_frames.clone();
    let recv_ref = mock.recv_queue.clone();

    let wrapper = BluetoothWrapper::new(Box::new(mock));
    wrapper.connect("11:22:33:44:55:66").unwrap();
    assert!(wrapper.is_connected());

    // Push an unsolicited XM4 capability notification (containing 0x3C to test escaping),
    // followed by an ACK with seq=1.
    let cap_payload = vec![
        CommandType::CapabilityQueryResponse as u8,
        0x01,
        0x3C,
        98,
        (-11i8) as u8,
    ];
    let notif = package_data_for_bt(&cap_payload, DataType::DataMdr, 0).unwrap();
    let ack = package_data_for_bt(&[], DataType::Ack, 1).unwrap();
    {
        let mut q = recv_ref.lock().unwrap();
        // Combine into a single recv chunk to verify back-to-back frame extraction
        let mut combined = notif;
        combined.extend(ack);
        q.push_back(combined);
    }

    let bytes_sent = wrapper.send_command(&[0x06, 0x01]).unwrap();
    assert!(bytes_sent > 0);
    assert_eq!(wrapper.seq_number().unwrap(), 1);

    let pending = wrapper.drain_pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].message_bytes, cap_payload);

    // Verify client sent both the command and the ACK (seq = 0 ^ 1 = 1) for the notification
    let sent = sent_ref.lock().unwrap();
    assert_eq!(sent.len(), 2);
}

#[test]
fn rejects_duplicate_start_marker() {
    let mock = MockBluetoothConnector::new(vec![]);
    mock.push_incoming(vec![b'>', b'a', b'>', b'<']);
    let wrapper = BluetoothWrapper::new(Box::new(mock));
    let err = wrapper.read_message().unwrap_err();
    assert!(err.should_disconnect());
    assert!(err.to_string().contains("Multiple start markers"));
}
