//! 快速陷入处理。

#![no_std]
#![feature(naked_functions)]
#![deny(warnings, missing_docs)]

mod entire;
mod fast;

pub use entire::*;
pub use fast::*;

use core::{alloc::Layout, arch::asm, marker::PhantomData, ptr::NonNull};

/// 陷入栈。
#[repr(transparent)]
pub struct TrapStack<B: AsRef<[u8]> + AsMut<[u8]>, T>(B, PhantomData<T>);

/// 构造陷入栈失败。
pub struct IllegalStack;

impl<B: AsRef<[u8]> + AsMut<[u8]>, T> TrapStack<B, T> {
    /// 将内存块配置为使用 `fast_handler` 完成快速路径的陷入栈。
    pub fn new(mut block: B, fast_handler: FastHandler<T>) -> Result<Self, IllegalStack> {
        let slice = block.as_mut();
        let ptr = Self::locate_ptr(slice);
        if ptr >= slice.as_ptr() as usize {
            unsafe { &mut *(ptr as *mut TrapHandler<T>) }.fast_handler = fast_handler;
            Ok(Self(block, PhantomData))
        } else {
            Err(IllegalStack)
        }
    }

    /// 将这个内核栈加载为预备陷入栈。返回 `sscratch` 寄存器中原本的值。
    ///
    /// # Safety
    ///
    /// 在 RISC-V 中，陷入栈被保存在 `sscratch` 寄存器中。
    /// 这个函数本质上是将当前栈的指针存入 `sscratch`，这会替换掉 `sscratch` 原来的值。
    /// 这个值很可能是重要的，保存它是用户的责任。
    #[inline]
    pub unsafe fn load(&self) -> usize {
        let mut sscratch = Self::locate_ptr(self.0.as_ref());
        asm!("csrrw {0}, sscratch, {0}", inlateout(reg) sscratch);
        sscratch
    }

    /// 在内存块上定位一个处理器上下文。
    #[inline]
    fn locate_ptr(slice: &[u8]) -> usize {
        let layout = Layout::new::<TrapHandler<T>>();
        (slice.as_ptr_range().end as usize - layout.size()) & !(layout.size() - 1)
    }
}

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

/// 陷入上下文指针。
#[repr(transparent)]
pub struct ContextPtr(NonNull<TrapContext>);

impl ContextPtr {
    /// 从上下文向硬件加载非调用规范约定的寄存器。
    #[inline]
    unsafe fn load_regs(&self) {
        let ctx = self.0.as_ref();
        asm!(
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

/// 设置全局陷入入口。
///
/// # Safety
///
/// 这个函数操作硬件寄存器，寄存器里原本的值将丢弃。
#[inline]
pub unsafe fn load_direct_trap_entry() {
    asm!("csrw stvec, {0}", in(reg) trap_entry, options(nomem))
}

/// 陷入处理例程。
///
/// # Safety
///
/// 不要直接调用这个函数。暴露它仅仅是为了提供其入口的符号链接。
#[naked]
pub unsafe extern "C" fn trap_entry() {
    asm!(
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
        // 加载上下文指针
        "0: ld    a1,  1*8(sp)",
        // 0：设置少量参数寄存器
        "   beqz  a0, 0f",
        // 1：设置所有参数寄存器
        "   addi  a0, a0, -1
            beqz  a0, 1f
        ",
        // 2：设置所有调用者寄存器
        "   addi  a0, a0, -1
            beqz  a0, 2f
        ",
        // 3：设置所有寄存器
        "   addi  a0, a0, -1
            beqz  a0, 3f
        ",
        // 4：完整路径
        "   ld    a0,  1*8(sp)
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
            j     0b
        ",
        // 设置所有寄存器
        "3: ld    s0, 16*8(a1)
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
        "1: ld    a2, 10*8(a1)
            ld    a3, 11*8(a1)
            ld    a4, 12*8(a1)
            ld    a5, 13*8(a1)
            ld    a6, 14*8(a1)
            ld    a7, 15*8(a1)
        ",
        // 设置少量参数寄存器
        "0: ld    a0,  8*8(a1)
            ld    a1,  9*8(a1)
            csrrw sp, sscratch, sp
            sret
        ",
        options(noreturn),
    )
}
