use crate::{EntireHandler, FlowContext, TrapHandler};
use core::{mem::MaybeUninit, ptr::NonNull};

/// 快速路径函数。
pub type FastHandler = extern "C" fn(
    ctx: FastContext,
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
pub struct FastContext(&'static mut TrapHandler);

impl FastContext {
    /// 访问陷入上下文的 a0 寄存器。
    ///
    /// 由于 a0 寄存器在快速路径中用于传递上下文指针，
    /// 将陷入上下文的 a0 暂存到陷入处理器上下文中。
    #[inline]
    pub fn a0(&self) -> usize {
        self.0.scratch
    }

    /// 获取控制流上下文。
    #[inline]
    pub fn regs(&mut self) -> &mut FlowContext {
        unsafe { self.0.context.as_mut() }
    }

    /// 交换上下文指针。
    #[inline]
    pub fn swap_context(&mut self, new: NonNull<FlowContext>) -> NonNull<FlowContext> {
        core::mem::replace(&mut self.0.context, new)
    }

    /// 启动一个带有 `argc` 个参数的新上下文。
    #[inline]
    pub fn call(self, argc: usize) -> FastResult {
        unsafe { self.0.context.as_ref().load_others() };
        if argc <= 2 {
            FastResult::FastCall
        } else {
            FastResult::Call
        }
    }

    /// 从快速路径恢复。
    ///
    /// > **NOTICE** 必须先手工调用 `save_args`，或通过其他方式设置参数寄存器。
    #[inline]
    pub fn restore(self) -> FastResult {
        FastResult::Restore
    }

    /// 丢弃当前上下文，并直接切换到另一个上下文。
    #[inline]
    pub fn switch_to(self, others: NonNull<FlowContext>) -> FastResult {
        unsafe { others.as_ref().load_others() };
        self.0.context = others;
        FastResult::Switch
    }

    /// 向完整路径 `f` 传递对象 `t`。
    ///
    /// > **NOTICE** 必须先手工调用 `save_args`，或通过其他方式设置参数寄存器。
    #[inline]
    pub fn continue_with<T: 'static>(self, f: EntireHandler<T>, t: T) -> FastResult {
        // TODO 检查栈溢出
        unsafe { *self.0.locate_fast_mail() = MaybeUninit::new(t) };
        self.0.scratch = f as _;
        FastResult::Continue
    }
}

/// 快速路径处理结果。
#[repr(usize)]
pub enum FastResult {
    /// 调用新上下文，只需设置 2 个或更少参数。
    FastCall = 0,
    /// 调用新上下文，需要设置超过 2 个参数。
    Call = 1,
    /// 从快速路径直接返回。
    Restore = 2,
    /// 直接切换到另一个上下文。
    Switch = 3,
    /// 调用完整路径函数。
    Continue = 4,
}
