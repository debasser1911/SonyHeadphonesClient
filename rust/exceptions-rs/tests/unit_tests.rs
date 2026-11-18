use exceptions_rs::port::HeadphonesError;

#[test]
fn recoverable_and_runtime_errors() {
    let rec = HeadphonesError::recoverable("Invalid checksum!", true);
    assert!(rec.is_recoverable());
    assert!(rec.should_disconnect());
    assert_eq!(rec.to_string(), "Invalid checksum!");

    let soft = HeadphonesError::recoverable("No radios", false);
    assert!(soft.is_recoverable());
    assert!(!soft.should_disconnect());

    let fatal = HeadphonesError::runtime("WSAStartup failed");
    assert!(!fatal.is_recoverable());
    assert!(!fatal.should_disconnect());
    assert_eq!(fatal.to_string(), "WSAStartup failed");
}
