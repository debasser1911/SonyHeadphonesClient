pub mod port;

// SAFETY: Exported C ABI symbol returning a scalar integer constant.
#[unsafe(no_mangle)]
pub extern "C" fn exceptions_error_kinds() -> i32 {
    2
}
