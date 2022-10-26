use crate::TrapHandler;

/// 完整路径函数。
pub type EntireHandler<T> = extern "C" fn(ctx: &mut EntireContext<T>) -> EntireResult;

/// 完整路径上下文。
#[repr(transparent)]
pub struct EntireContext<T>(TrapHandler<T>);

/// 完整路径处理结果。
#[repr(usize)]
pub enum EntireResult {
    /// 切换到另一个上下文或从完整路径恢复。
    Restore = 1,
    /// 调用新上下文，需要设置超过 2 个参数。
    ComplexCall = 3,
    /// 调用新上下文，只需设置 2 个或更少参数。
    Call = 4,
}
