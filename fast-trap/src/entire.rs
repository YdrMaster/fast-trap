use crate::TrapHandler;
use core::{
    marker::PhantomData,
    mem::forget,
    ops::{Deref, DerefMut},
    ptr::{drop_in_place, NonNull},
};

/// 完整路径函数。
pub type EntireHandler<T> = extern "C" fn(EntireContext<T>) -> EntireResult;

/// 完整路径上下文。
#[repr(transparent)]
pub struct EntireContext<T: 'static = ()>(NonNull<TrapHandler>, PhantomData<T>);

/// 分离了快速路径消息的完整路径上下文。
#[repr(transparent)]
pub struct EntireContextSeparated(&'static mut TrapHandler);

impl<T: 'static> EntireContext<T> {
    /// 分离完整路径上下文和快速路径消息。
    #[inline]
    pub fn split(self) -> (EntireContextSeparated, FastMail<T>) {
        let mail = unsafe { &mut *self.0.as_ref().locate_fast_mail() };
        let mut handler = self.0;
        forget(self);
        (
            EntireContextSeparated(unsafe { handler.as_mut() }),
            FastMail(mail),
        )
    }
}

/// 如果没有调用分离，快速路径消息对象可以直接释放。
impl<T: 'static> Drop for EntireContext<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { drop_in_place(self.0.as_ref().locate_fast_mail::<T>()) }
    }
}

/// 快速路径消息。
#[repr(transparent)]
pub struct FastMail<T: 'static>(&'static mut T);

impl<T: 'static> Deref for FastMail<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: 'static> DerefMut for FastMail<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// 快速路径消息如果未被取走，将自动释放。
impl<T: 'static> Drop for FastMail<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { drop_in_place(self.0) }
    }
}

/// 完整路径处理结果。
#[repr(usize)]
pub enum EntireResult {
    /// 调用新上下文，只需设置 2 个或更少参数。
    FastCall = 0,
    /// 调用新上下文，需要设置超过 2 个参数。
    Call = 1,
    /// 切换到另一个上下文或从完整路径恢复。
    Restore = 4,
}
