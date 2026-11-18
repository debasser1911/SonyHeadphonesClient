pub mod port;

// SAFETY: Exported C ABI scalar function performing pure arithmetic.
#[unsafe(no_mangle)]
pub extern "C" fn byteOrderSwap(num: std::ffi::c_uint) -> std::ffi::c_uint {
    port::byte_order_swap(num)
}
