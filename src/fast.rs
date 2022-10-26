use crate::{ContextPtr, TrapHandler};

/// 快速路径函数。
pub type FastHandler<T> = extern "C" fn(
    ctx: &mut FastContext<T>,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> FastResult;

/// 快速路径上下文。
///
/// 将陷入处理器上下文中在快速路径中可安全操作的部分暴露给快速路径函数。
#[repr(transparent)]
pub struct FastContext<T>(TrapHandler<T>);

impl<T> FastContext<T> {
    /// 访问陷入上下文的 a0 寄存器。
    ///
    /// 由于 a0 寄存器在快速路径中用于传递上下文指针，
    /// 将陷入上下文的 a0 暂存到陷入处理器上下文中。
    #[inline]
    pub fn a0(&self) -> usize {
        self.0.scratch
    }

    /// 访问陷入上下文的临时寄存器组。
    #[inline]
    pub fn t(&self, i: usize) -> usize {
        unsafe { self.0.context.0.as_ref() }.t[i]
    }

    /// 将所有参数寄存器保存到陷入上下文。
    #[inline]
    pub extern "C" fn save_args(
        &mut self,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
        a6: usize,
        a7: usize,
    ) {
        let ctx = unsafe { self.0.context.0.as_mut() };
        ctx.a[0] = self.a0();
        ctx.a[1] = a1;
        ctx.a[2] = a2;
        ctx.a[3] = a3;
        ctx.a[4] = a4;
        ctx.a[5] = a5;
        ctx.a[6] = a6;
        ctx.a[7] = a7;
    }

    /// 向完整路径传递对象 `t`。
    ///
    /// > **NOTICE** 必须先手工调用 `save_args`，或通过其他方式设置参数寄存器。
    #[inline]
    pub fn continue_with(&mut self, t: T) -> FastResult {
        self.0.extra = t;
        FastResult::Continue
    }

    /// 从快速路径恢复。
    ///
    /// > **NOTICE** 必须先手工调用 `save_args`，或通过其他方式设置参数寄存器。
    #[inline]
    pub fn restore(&mut self) -> FastResult {
        FastResult::Restore
    }

    /// 丢弃当前上下文，并直接切换到另一个上下文。
    #[inline]
    pub fn switch_to(&mut self, others: ContextPtr) -> FastResult {
        unsafe { others.load_regs() };
        self.0.context = others;
        FastResult::Switch
    }

    /// 丢弃当前上下文，并启动一个带有 `argc` 个参数的新上下文。
    #[inline]
    pub fn call(&mut self, new: ContextPtr, argc: usize) -> FastResult {
        unsafe { new.load_regs() };
        self.0.context = new;
        if argc > 2 {
            FastResult::ComplexCall
        } else {
            FastResult::Call
        }
    }
}

/// 快速路径处理结果。
#[repr(usize)]
pub enum FastResult {
    /// 调用完整路径函数。
    Continue = 0,
    /// 直接切换到另一个上下文。
    Switch = 1,
    /// 从快速路径直接返回。
    Restore = 2,
    /// 调用新上下文，需要设置超过 2 个参数。
    ComplexCall = 3,
    /// 调用新上下文，只需设置 2 个或更少参数。
    Call = 4,
}
