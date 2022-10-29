#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

use console::log;
use core::{arch::asm, ptr::NonNull};
use fast_trap::{
    load_direct_trap_entry, trap_entry, FastContext, FastResult, FlowContext, FreeTrapStack,
    TrapStackBlock,
};
use riscv::register::*;
use sbi_rt::*;

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096;

    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    asm!(
        "   la   sp, {stack} + {stack_size}
            call {main}
         2: la   ra, 1f
            csrw sepc, ra
            j    {trap}
         1: j    2b
        ",
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
        trap       =   sym trap_entry,
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
static mut ROOT_CONTEXT: FlowContext = FlowContext::ZERO;

extern "C" fn rust_main() {
    console::init_console(&Console);
    console::set_log_level(option_env!("LOG"));
    console::test_log();

    sscratch::write(0x5050);
    let context_ptr = unsafe { NonNull::new_unchecked(&mut ROOT_CONTEXT) };

    // 测试构造和释放
    let _ = FreeTrapStack::new(unsafe { &mut ROOT_STACK }, context_ptr, fast_handler).unwrap();
    assert_eq!(0x5050, sscratch::read());

    // 测试加载和卸载
    let _ = FreeTrapStack::new(unsafe { &mut ROOT_STACK }, context_ptr, fast_handler)
        .unwrap()
        .load();
    assert_eq!(0x5050, sscratch::read());

    // 加载一个新的陷入栈
    let _loaded = FreeTrapStack::new(unsafe { &mut ROOT_STACK }, context_ptr, fast_handler)
        .unwrap()
        .load();
    assert_ne!(0x5050, sscratch::read());
    log::debug!("sscratch: {:#x}", sscratch::read());

    // 测试直接调用
    unsafe {
        asm!(
            "   la   {0}, 1f
                csrw sepc, {0}
                li   {0}, 31
                csrw scause, {0}
                j    {trap}
             1:
            ",
            out(reg) _,
            trap = sym trap_entry,
        );
    }

    // 测试内核异常
    unsafe {
        load_direct_trap_entry();
        asm!("unimp");
    }

    loop {}
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
    use {scause::Exception as E, scause::Trap as T};
    let cause = scause::read();
    match cause.cause() {
        T::Exception(E::IllegalInstruction) => {
            let mut sepc = sepc::read();
            sepc += 2;
            sepc::write(sepc);
        }
        T::Exception(_) | T::Interrupt(_) => {}
    }
    log::debug!("fast trap: {:?}({})", cause.cause(), cause.bits());
    ctx.save_args(a1, a2, a3, a4, a5, a6, a7);
    unsafe { sstatus::set_spp(sstatus::SPP::Supervisor) };
    ctx.restore()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    system_reset(Shutdown, SystemFailure);
    loop {}
}

pub struct Console;

impl console::Console for Console {
    #[inline]
    fn put_char(&self, c: u8) {
        #[allow(deprecated)]
        sbi_rt::legacy::console_putchar(c as _);
    }
}
