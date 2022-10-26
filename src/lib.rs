//! 快速陷入处理。

#![no_std]
#![feature(naked_functions)]
#![deny(warnings, missing_docs)]

mod entire;
mod fast;

pub use entire::*;
pub use fast::*;

use core::{alloc::Layout, ptr::NonNull};

/// 陷入处理器上下文。
#[repr(C)]
pub struct TrapHandler<T> {
    /// 在快速路径中暂存 a0。
    scratch: usize,
    /// 指向一个陷入上下文的指针。
    ///
    /// - 发生陷入时，将寄存器保存到此对象。
    /// - 离开陷入处理时，按此对象的内容设置寄存器。
    context: ContextPtr,
    /// 快速路径函数。
    ///
    /// 必须在初始化陷入时设置好。
    fast_handler: FastHandler<T>,
    /// 完整路径函数。
    ///
    /// 可以在初始化陷入时设置好，也可以在快速路径中设置。
    entire_handler: EntireHandler<T>,
    /// 补充信息，用于从快速路径向完整路径传递信息。
    extra: T,
}

impl<T> TrapHandler<T> {
    const LAYOUT: Layout = Layout::new::<Self>();

    /// 在一个栈帧上初始化陷入处理器上下文，并返回上下文指针。
    #[inline]
    pub fn new_in(stack: &mut [u8], fast_handler: FastHandler<T>) -> TrapHandlerPtr<T> {
        let top = stack.as_ptr_range().end as usize;
        assert!(top.trailing_zeros() > Self::LAYOUT.align().trailing_zeros());

        let ptr = (top - Self::LAYOUT.size()) & !(Self::LAYOUT.align() - 1);
        let mut ptr = NonNull::new(ptr as *mut Self).unwrap();
        unsafe { ptr.as_mut() }.fast_handler = fast_handler;

        TrapHandlerPtr(ptr)
    }
}

/// 指向一个陷入上下文的指针。
///
/// 通过这个指针还能找到存放上下文的栈的高地址。
#[repr(transparent)]
pub struct TrapHandlerPtr<T>(NonNull<TrapHandler<T>>);

impl<T> TrapHandlerPtr<T> {
    const LAYOUT: Layout = TrapHandler::<T>::LAYOUT;

    /// 通过上下文指针找到栈顶。
    #[inline]
    pub fn stack_top(&self) -> usize {
        let mask = Self::LAYOUT.align() - 1;
        (self.0.as_ptr() as usize + Self::LAYOUT.size() + mask) & !mask
    }
}

/// 陷入上下文指针。
#[repr(transparent)]
pub struct ContextPtr(NonNull<TrapContext>);

impl ContextPtr {
    /// 从上下文向硬件加载非调用规范约定的寄存器。
    #[inline]
    unsafe fn load_regs(&self) {
        let ctx = self.0.as_ref();
        core::arch::  asm!(
            "   mv         gp, {gp}
                mv         tp, {tp}
                csrw sscratch, {sp}
                csrw     sepc, {pc}
            ",
            gp = in(reg) ctx.gp,
            tp = in(reg) ctx.tp,
            sp = in(reg) ctx.sp,
            pc = in(reg) ctx.pc,
        );
    }
}

/// 陷入上下文。
///
/// 保存了陷入时的寄存器状态。包括所有通用寄存器和 `pc`。
#[repr(C)]
#[allow(missing_docs)]
pub struct TrapContext {
    pub ra: usize,      // 0..
    pub t: [usize; 7],  // 1..
    pub a: [usize; 8],  // 8..
    pub s: [usize; 12], // 16..
    pub gp: usize,      // 28..
    pub tp: usize,      // 29..
    pub sp: usize,      // 30..
    pub pc: usize,      // 31..
}

/// 陷入入口地址。
#[inline]
pub fn trap_entry() -> usize {
    trap as _
}

#[naked]
unsafe extern "C" fn trap() {
    core::arch::asm!(
        ".align 2",
        // 换栈
        "   csrrw sp, sscratch, sp",
        // 加载上下文指针
        "   sd    a0,  0*8(sp)
            ld    a0,  1*8(sp)
        ",
        // 保存尽量少的寄存器
        "   sd    ra,  0*8(a0)
            sd    t0,  1*8(a0)
            sd    t1,  2*8(a0)
            sd    t2,  3*8(a0)
            sd    t3,  4*8(a0)
            sd    t4,  5*8(a0)
            sd    t5,  6*8(a0)
            sd    t6,  7*8(a0)
        ",
        // 调用快速路径函数
        //
        // | reg    | position
        // | ------ | -
        // | ra     | `TrapHandler.context`
        // | t0-t6  | `TrapHandler.context`
        // | a0     | `TrapHandler.scratch`
        // | a1-a7  | 参数寄存器
        // | sp     | sscratch
        // | gp, tp | gp, tp
        // | s0-s11 | 不支持
        //
        // > 若要保留陷入上下文，
        // > 必须在快速路径保存 a0-a7 到 `TrapHandler.context`，
        // > 并进入完整路径执行后续操作。
        // >
        // > 若要切换上下文，在快速路径设置 gp/tp/sscratch/sepc 和 sstatus。
        "   mv    a0,      sp
            ld    ra,  2*8(sp)
            jalr  ra
        ",
        "   beqz  a0, 0f",
        // 加载上下文指针
        "   ld    a1,  1*8(sp)",
        // 1：设置所有寄存器
        "1: addi  a0, a0, -1
            beqz  a0, 1f
        ",
        // 2：设置调用者寄存器
        "   addi  a0, a0, -1
            beqz  a0, 2f
        ",
        // 3：设置所有参数寄存器
        "   addi  a0, a0, -1
            beqz  a0, 3f
        ",
        // 4：设置少量参数寄存器
        "   addi  a0, a0, -1
            beqz  a0, 4f
        ",
        // unreachable!
        "   unimp",
        // 保存其他寄存器
        "0: ld    a0,  1*8(sp)
            sd    s0, 16*8(a0)
            sd    s1, 17*8(a0)
            sd    s2, 18*8(a0)
            sd    s3, 19*8(a0)
            sd    s4, 20*8(a0)
            sd    s5, 21*8(a0)
            sd    s6, 22*8(a0)
            sd    s7, 23*8(a0)
            sd    s8, 24*8(a0)
            sd    s9, 25*8(a0)
            sd    s10,26*8(a0)
            sd    s11,27*8(a0)
        ",
        // 调用完整路径函数
        //
        // | reg    | position
        // | ------ | -
        // | sp     | sscratch
        // | gp, tp | gp, tp
        // | else   | `TrapHandler.context`
        //
        // > 若要保留陷入上下文，
        // > 在完整路径中保存 gp/tp/sp/pc 到 `TrapHandler.context`。
        // >
        // > 若要切换上下文，在完整路径设置 gp/tp/sscratch/sepc 和 sstatus。
        "   mv    a0,      sp
            ld    ra,  3*8(sp)
            jalr  ra
            j     1b
        ",
        // 恢复所有寄存器
        "1: ld    s0, 16*8(a1)
            ld    s1, 17*8(a1)
            ld    s2, 18*8(a1)
            ld    s3, 19*8(a1)
            ld    s4, 20*8(a1)
            ld    s5, 21*8(a1)
            ld    s6, 22*8(a1)
            ld    s7, 23*8(a1)
            ld    s8, 24*8(a1)
            ld    s9, 25*8(a1)
            ld    s10,26*8(a1)
            ld    s11,27*8(a1)
        ",
        // 设置所有调用者寄存器
        "2: ld    ra,  0*8(a1)
            ld    t0,  1*8(a1)
            ld    t1,  2*8(a1)
            ld    t2,  3*8(a1)
            ld    t3,  4*8(a1)
            ld    t4,  5*8(a1)
            ld    t5,  6*8(a1)
            ld    t6,  7*8(a1)
        ",
        // 设置所有参数寄存器
        "3: ld    a2, 10*8(a1)
            ld    a3, 11*8(a1)
            ld    a4, 12*8(a1)
            ld    a5, 13*8(a1)
            ld    a6, 14*8(a1)
            ld    a7, 15*8(a1)
        ",
        // 设置少量参数寄存器
        "4: ld    a0,  8*8(a1)
            ld    a1,  9*8(a1)
            csrrw sp, sscratch, sp
            sret
        ",
        options(noreturn),
    )
}
