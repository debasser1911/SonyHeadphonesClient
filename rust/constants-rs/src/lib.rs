pub mod port;

// SAFETY: Exported C ABI symbol taking/returning only scalar integer values.
#[unsafe(no_mangle)]
pub extern "C" fn constants_max_steps_xm4() -> i32 {
    i32::from(port::MAX_STEPS_WH_1000_XM4)
}
