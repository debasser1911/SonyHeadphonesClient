pub mod port;

// SAFETY: Exported C ABI scalar symbol returning a constant integer.
#[unsafe(no_mangle)]
pub extern "C" fn bt_wrapper_max_msg_size() -> std::ffi::c_int {
    constants_rs::port::MAX_BLUETOOTH_MESSAGE_SIZE as std::ffi::c_int
}
