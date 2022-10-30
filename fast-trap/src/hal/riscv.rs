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
