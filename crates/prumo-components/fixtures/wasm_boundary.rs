//! Rust-only boundary fixture. Not an engineering implementation or extension.
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo<'_>) -> ! {
    loop { core::hint::spin_loop(); }
}

// SAFETY: fixed symbols in a private Wasm module, with scalar-only ABI.
// No pointers, shared memory, ownership transfer or foreign callbacks exist.
#[unsafe(no_mangle)]
pub extern "C" fn prumo_contract_version() -> u32 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn millimetres_to_metres(value: f64) -> f64 { value / 1000.0 }

#[unsafe(no_mangle)]
pub extern "C" fn request_memory(pages: usize) -> usize {
    core::arch::wasm32::memory_grow::<0>(pages)
}

#[unsafe(no_mangle)]
pub extern "C" fn exhaust_fuel() -> ! {
    loop { core::hint::spin_loop(); }
}
