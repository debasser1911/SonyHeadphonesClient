use bluetooth_connector_rs::port::*;

#[test]
fn mock_connector_connects_and_transfers_bytes() {
    let mut mock = MockBluetoothConnector::new(vec![BluetoothDevice {
        name: "WH-1000XM4".into(),
        mac: "11:22:33:44:55:66".into(),
    }]);
    assert!(!mock.is_connected());
    assert_eq!(mock.get_connected_devices().unwrap().len(), 1);
    assert!(mock.send(b"hello").is_err());

    mock.connect("11:22:33:44:55:66").unwrap();
    assert!(mock.is_connected());
    assert_eq!(mock.send(b"hello").unwrap(), 5);

    mock.push_incoming(vec![1, 2, 3, 4]);
    let mut buf = [0u8; 2];
    assert_eq!(mock.recv(&mut buf).unwrap(), 2);
    assert_eq!(buf, [1, 2]);
    assert_eq!(mock.recv(&mut buf).unwrap(), 2);
    assert_eq!(buf, [3, 4]);

    mock.disconnect();
    assert!(!mock.is_connected());
}

#[cfg(target_os = "windows")]
#[test]
fn windows_connector_constructs_and_queries_radios_without_panicking() {
    let conn = win::WindowsBluetoothConnector::new().unwrap();
    assert!(!conn.is_connected());
    // Either returns radios/devices or a clean Recoverable/Runtime error (never panics)
    let _ = conn.get_connected_devices();
}
