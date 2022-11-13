mod riscv;
#[cfg(feature = "riscv-m")]
#[macro_use]
mod riscv_m;
#[cfg(feature = "riscv-s")]
#[macro_use]
mod riscv_s;

pub use riscv::*;
#[cfg(feature = "riscv-m")]
pub use riscv_m::*;
#[cfg(feature = "riscv-s")]
pub use riscv_s::*;
