use crate::TrapHandler;
use core::alloc::Layout;

/// 陷入上下文。
///
/// 保存了陷入时的寄存器状态。包括所有通用寄存器和 `pc`。
#[repr(C)]
#[allow(missing_docs)]
pub struct FlowContext {
    pub ra: usize,      // 0..
    pub t: [usize; 7],  // 1..
    pub a: [usize; 8],  // 8..
    pub s: [usize; 12], // 16..
    pub gp: usize,      // 28..
    pub tp: usize,      // 29..
    pub sp: usize,      // 30..
    pub pc: usize,      // 31..
}

impl FlowContext {
    /// 零初始化。
    pub const ZERO: Self = Self {
        ra: 0,
        t: [0; 7],
        a: [0; 8],
        s: [0; 12],
        gp: 0,
        tp: 0,
        sp: 0,
        pc: 0,
    };
}

/// 把当前栈复用为陷入栈，预留 Handler 空间。
///
/// # Safety
///
/// 裸指针，直接移动 sp，只能在纯汇编环境调用。
#[naked]
pub unsafe extern "C" fn reuse_stack_for_trap() {
    const LAYOUT: Layout = Layout::new::<TrapHandler>();
    core::arch::asm!(
        "   addi sp, sp, {size}
            andi sp, sp, {mask}
            ret
        ",
        size = const -(LAYOUT.size() as isize),
        mask = const !(LAYOUT.align() as isize - 1) ,
        options(noreturn)
    )
}
