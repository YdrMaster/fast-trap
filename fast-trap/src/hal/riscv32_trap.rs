use super::{exchange, r#return};

/// 陷入处理例程。
///
/// # Safety
///
/// 不要直接调用这个函数。暴露它仅仅是为了提供其入口的符号链接。
#[naked]
pub unsafe extern "C" fn trap_entry() {
    core::arch::asm!(
        ".align 2",
        // 换栈
        exchange!(),
        // 加载上下文指针
        "   sw    a0,  2*8(sp)
            lw    a0,  0*8(sp)
        ",
        // 保存尽量少的寄存器
        "   sw    ra,  0*8(a0)
            sw    t0,  1*8(a0)
            sw    t1,  2*8(a0)
            sw    t2,  3*8(a0)
            sw    t3,  4*8(a0)
            sw    t4,  5*8(a0)
            sw    t5,  6*8(a0)
            sw    t6,  7*8(a0)
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
            lw    ra,  1*8(sp)
            jalr  ra
        ",
        // 加载上下文指针
        "0: lw    a1,  0*8(sp)",
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
        "   sw    s0, 16*8(a1)
            sw    s1, 17*8(a1)
            sw    s2, 18*8(a1)
            sw    s3, 19*8(a1)
            sw    s4, 20*8(a1)
            sw    s5, 21*8(a1)
            sw    s6, 22*8(a1)
            sw    s7, 23*8(a1)
            sw    s8, 24*8(a1)
            sw    s9, 25*8(a1)
            sw    s10,26*8(a1)
            sw    s11,27*8(a1)
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
            lw    ra,  2*8(sp)
            jalr  ra
            j     0b
        ",
        // 设置所有寄存器
        "3: lw    s0, 16*8(a1)
            lw    s1, 17*8(a1)
            lw    s2, 18*8(a1)
            lw    s3, 19*8(a1)
            lw    s4, 20*8(a1)
            lw    s5, 21*8(a1)
            lw    s6, 22*8(a1)
            lw    s7, 23*8(a1)
            lw    s8, 24*8(a1)
            lw    s9, 25*8(a1)
            lw    s10,26*8(a1)
            lw    s11,27*8(a1)
        ",
        // 设置所有调用者寄存器
        "2: lw    ra,  0*8(a1)
            lw    t0,  1*8(a1)
            lw    t1,  2*8(a1)
            lw    t2,  3*8(a1)
            lw    t3,  4*8(a1)
            lw    t4,  5*8(a1)
            lw    t5,  6*8(a1)
            lw    t6,  7*8(a1)
        ",
        // 设置所有参数寄存器
        "1: lw    a2, 10*8(a1)
            lw    a3, 11*8(a1)
            lw    a4, 12*8(a1)
            lw    a5, 13*8(a1)
            lw    a6, 14*8(a1)
            lw    a7, 15*8(a1)
        ",
        // 设置少量参数寄存器
        "0: lw    a0,  8*8(a1)
            lw    a1,  9*8(a1)
        ",
        exchange!(),
        r#return!(),
        options(noreturn),
    )
}
