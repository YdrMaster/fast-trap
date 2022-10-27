#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

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

extern "C" fn rust_main() -> ! {
    use sbi_rt::*;
    for c in b"Hello, world!" {
        #[allow(deprecated)]
        legacy::console_putchar(*c as _);
    }
    system_reset(Shutdown, NoReason);
    unreachable!()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    use sbi_rt::*;
    system_reset(Shutdown, SystemFailure);
    loop {}
}
