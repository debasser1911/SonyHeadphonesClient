pub mod port;

// SAFETY: Exported C ABI scalar symbol returning a constant integer.
#[unsafe(no_mangle)]
pub extern "C" fn headphones_xm4_max_steps() -> std::ffi::c_int {
    std::ffi::c_int::from(constants_rs::port::MAX_STEPS_WH_1000_XM4)
}
