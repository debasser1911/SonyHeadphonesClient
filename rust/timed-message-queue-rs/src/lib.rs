pub mod port;

// SAFETY: Exported C ABI symbol returning a scalar integer constant.
#[unsafe(no_mangle)]
pub extern "C" fn tmq_default_duration_ms() -> u64 {
    port::DEFAULT_ERROR_MESSAGE_MS
}
