#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

use core::mem::forget;
use fast_trap::{FastContext, FastResult, FreeTrapStack, TrapStackBlock};
use sbi_rt::*;

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096;

    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
        options(noreturn),
    )
}

#[repr(C, align(4096))]
struct Stack([u8; 4096]);

impl AsRef<[u8]> for Stack {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Stack {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl TrapStackBlock for &'static mut Stack {}

static mut ROOT_STACK: Stack = Stack([0; 4096]);

extern "C" fn rust_main() -> ! {
    for c in b"Hello, world!" {
        #[allow(deprecated)]
        legacy::console_putchar(*c as _);
    }
    forget(
        FreeTrapStack::new(unsafe { &mut ROOT_STACK }, fast_handler)
            .unwrap()
            .load(),
    );
    system_reset(Shutdown, NoReason);
    unreachable!()
}

extern "C" fn fast_handler(
    mut ctx: FastContext,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> FastResult {
    for c in b"Hello, world!\n" {
        #[allow(deprecated)]
        legacy::console_putchar(*c as _);
    }
    ctx.save_args(a1, a2, a3, a4, a5, a6, a7);
    ctx.restore()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    system_reset(Shutdown, SystemFailure);
    loop {}
}
