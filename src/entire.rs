use core::{marker::PhantomData, ptr::drop_in_place};

use crate::TrapHandler;

/// 完整路径函数。
pub type EntireHandler<T> = extern "C" fn(EntireContext<T>) -> EntireResult;

/// 完整路径上下文。
#[repr(transparent)]
pub struct EntireContext<T: 'static = ()>(&'static mut TrapHandler, PhantomData<T>);

impl<T: 'static> EntireContext<T> {
    /// 获取从快速路径传来的信息。
    #[inline]
    pub fn fast_mail(&self) -> &T {
        unsafe { &*(self.0.range.start as *const T) }
    }

    /// 修改从快速路径传来的信息。
    #[inline]
    pub fn fast_mail_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.0.range.start as *mut T) }
    }
}

impl<T: 'static> Drop for EntireContext<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { drop_in_place(self.0.range.start as *mut T) }
    }
}

/// 完整路径处理结果。
#[repr(usize)]
pub enum EntireResult {
    /// 调用新上下文，只需设置 2 个或更少参数。
    Call = 0,
    /// 调用新上下文，需要设置超过 2 个参数。
    ComplexCall = 1,
    /// 切换到另一个上下文或从完整路径恢复。
    Restore = 4,
}
