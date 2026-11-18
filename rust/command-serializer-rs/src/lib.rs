pub mod port;

// SAFETY: Exported C ABI scalar function returning a constant integer.
#[unsafe(no_mangle)]
pub extern "C" fn serializer_max_steps_xm3() -> std::ffi::c_int {
    std::ffi::c_int::from(constants_rs::port::MAX_STEPS_WH_1000_XM3)
}
