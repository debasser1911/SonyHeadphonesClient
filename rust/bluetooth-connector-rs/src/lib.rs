pub mod port;

// SAFETY: Exported C ABI scalar symbol returning a constant indicator.
#[unsafe(no_mangle)]
pub extern "C" fn bt_connector_mock_available() -> std::ffi::c_int {
    1
}
