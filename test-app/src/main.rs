#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

use core::{
    arch::asm,
    mem::{forget, MaybeUninit},
    ptr::NonNull,
};
use fast_trap::{
    load_direct_trap_entry, reuse_stack_for_trap, trap_entry, FastContext, FastResult, FlowContext,
    FreeTrapStack, TrapStackBlock,
};
use rcore_console::log;
use riscv::register::*;
use uart_16550::MmioSerialPort;

#[link_section = ".bss.uninit"]
static mut ROOT_STACK: Stack = Stack([0; 4096]);
static mut ROOT_CONTEXT: FlowContext = FlowContext::ZERO;

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(
        "   la   sp, {stack} + {stack_size}
            call {move_stack}
            call {main}
            j    {trap}
        ",
        stack_size = const 4096,
        stack      =   sym ROOT_STACK,
        move_stack =   sym reuse_stack_for_trap,
        main       =   sym rust_main,
        trap       =   sym trap_entry,
        options(noreturn),
    )
}

#[naked]
unsafe extern "C" fn exception() -> ! {
    asm!("unimp", options(noreturn),)
}

extern "C" fn rust_main() {
    // 清零 bss 段
    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };
    // 初始化打印
    unsafe { UART = MaybeUninit::new(MmioSerialPort::new(0x1000_0000)) };
    rcore_console::init_console(&Console);
    rcore_console::set_log_level(option_env!("LOG"));
    rcore_console::test_log();

    #[cfg(feature = "m-mode")]
    mscratch::write(0x5050);
    #[cfg(feature = "s-mode")]
    sscratch::write(0x5050);
    let context_ptr = unsafe { NonNull::new_unchecked(&mut ROOT_CONTEXT) };

    // 测试构造和释放
    let _ = FreeTrapStack::new(
        StackRef(unsafe { &mut ROOT_STACK }),
        context_ptr,
        fast_handler,
    )
    .unwrap();
    #[cfg(feature = "m-mode")]
    assert_eq!(0x5050, mscratch::read());
    #[cfg(feature = "s-mode")]
    assert_eq!(0x5050, sscratch::read());

    // 测试加载和卸载
    let _ = FreeTrapStack::new(
        StackRef(unsafe { &mut ROOT_STACK }),
        context_ptr,
        fast_handler,
    )
    .unwrap()
    .load();
    #[cfg(feature = "m-mode")]
    assert_eq!(0x5050, mscratch::read());
    #[cfg(feature = "s-mode")]
    assert_eq!(0x5050, sscratch::read());

    // 加载一个新的陷入栈
    let loaded = FreeTrapStack::new(
        StackRef(unsafe { &mut ROOT_STACK }),
        context_ptr,
        fast_handler,
    )
    .unwrap()
    .load();

    #[cfg(feature = "m-mode")]
    {
        assert_ne!(0x5050, mscratch::read());
        log::debug!("mscratch: {:#x}", mscratch::read());
        unsafe { asm!("csrw mcause, {}", in(reg) 24) };
    }
    #[cfg(feature = "s-mode")]
    {
        assert_ne!(0x5050, sscratch::read());
        log::debug!("sscratch: {:#x}", sscratch::read());
        unsafe { asm!("csrw scause, {}", in(reg) 24) };
    }

    // 忘了它，在汇编里触发陷入还要用
    forget(loaded);

    // 加载陷入入口
    unsafe { load_direct_trap_entry() };
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
    #[cfg(feature = "m-mode")]
    {
        use {mcause::Exception as E, mcause::Trap as T};
        let cause = mcause::read();
        log::debug!("fast trap: {:?}({})", cause.cause(), cause.bits());
        match cause.cause() {
            T::Exception(E::IllegalInstruction) => {
                log::info!("Test pass");
                loop {}
            }
            T::Exception(E::Unknown) => {
                mepc::write(exception as _);
                unsafe { mstatus::set_mpp(mstatus::MPP::Machine) };
                ctx.save_args(a1, a2, a3, a4, a5, a6, a7);
                ctx.restore()
            }
            T::Exception(_) | T::Interrupt(_) => unreachable!(),
        }
    }
    #[cfg(feature = "s-mode")]
    {
        use {scause::Exception as E, scause::Trap as T};
        let cause = scause::read();
        log::debug!("fast trap: {:?}({})", cause.cause(), cause.bits());
        match cause.cause() {
            T::Exception(E::IllegalInstruction) => {
                log::info!("Test pass");
                loop {}
            }
            T::Exception(E::Unknown) => {
                sepc::write(exception as _);
                unsafe { sstatus::set_spp(sstatus::SPP::Supervisor) };
                ctx.save_args(a1, a2, a3, a4, a5, a6, a7);
                ctx.restore()
            }
            T::Exception(_) | T::Interrupt(_) => unreachable!(),
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}

#[repr(C, align(4096))]
struct Stack([u8; 4096]);

struct StackRef(&'static mut Stack);

impl AsRef<[u8]> for StackRef {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0 .0
    }
}

impl AsMut<[u8]> for StackRef {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0 .0
    }
}

impl TrapStackBlock for StackRef {}

impl Drop for StackRef {
    fn drop(&mut self) {
        log::info!("Stack Dropped!")
    }
}

struct Console;
static mut UART: MaybeUninit<MmioSerialPort> = MaybeUninit::uninit();

impl rcore_console::Console for Console {
    #[inline]
    fn put_char(&self, c: u8) {
        unsafe { UART.assume_init_mut() }.send(c);
    }
}
