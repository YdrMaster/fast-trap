//! 快速陷入处理。

#![no_std]
#![feature(naked_functions, asm_const)]
#![deny(warnings, missing_docs)]

mod entire;
mod fast;
mod hal;

pub use entire::*;
pub use fast::*;
pub use hal::*;

use core::{
    alloc::Layout,
    marker::PhantomPinned,
    mem::{align_of, forget},
    ops::Range,
    ptr::{drop_in_place, NonNull},
};

const TARGET: &str = "fast-trap";

/// 游离的陷入栈。
pub struct FreeTrapStack(NonNull<TrapHandler>);

/// 已加载的陷入栈。
pub struct LoadedTrapStack(usize);

/// 构造陷入栈失败。
#[derive(Debug)]
pub struct IllegalStack;

impl FreeTrapStack {
    /// 在内存块上构造游离的陷入栈。
    pub fn new(
        block: impl TrapStackBlock,
        context_ptr: NonNull<FlowContext>,
        fast_handler: FastHandler,
    ) -> Result<Self, IllegalStack> {
        const LAYOUT: Layout = Layout::new::<TrapHandler>();
        let range = block.as_ref().as_ptr_range();
        let bottom = range.start as usize;
        let top = range.end as usize;
        let ptr = (top - LAYOUT.size()) & !(LAYOUT.align() - 1);
        if ptr >= bottom {
            let handler = unsafe { &mut *(ptr as *mut TrapHandler) };
            handler.context = context_ptr;
            handler.fast_handler = fast_handler;
            handler.block = NonNull::from(&block);
            forget(block);
            log::trace!(target: TARGET, "new TrapStack({:?})", range);
            Ok(Self(unsafe { NonNull::new_unchecked(handler) }))
        } else {
            Err(IllegalStack)
        }
    }

    /// 将这个陷入栈加载为预备陷入栈。
    #[inline]
    pub fn load(self) -> LoadedTrapStack {
        log::trace!("load TrapStack({:#x?})", unsafe { self.0.as_ref().range() });
        let scratch = exchange_scratch(self.0.as_ptr() as _);
        forget(self);
        LoadedTrapStack(scratch)
    }
}

impl Drop for FreeTrapStack {
    #[inline]
    fn drop(&mut self) {
        log::trace!("delete TrapStack({:#x?})", unsafe {
            self.0.as_ref().range()
        });
        unsafe { drop_in_place(self.0.as_ref().block.as_ptr()) }
    }
}

impl LoadedTrapStack {
    /// 获取从 `sscratch` 寄存器中换出的值。
    #[inline]
    pub const fn val(&self) -> usize {
        self.0
    }

    /// 卸载陷入栈。
    #[inline]
    pub fn unload(self) -> FreeTrapStack {
        let ans = unsafe { self.unload_unchecked() };
        forget(self);
        ans
    }

    /// 卸载但不消费所有权。
    ///
    /// # Safety
    ///
    /// 间接复制了所有权。用于 `Drop`。
    #[inline]
    unsafe fn unload_unchecked(&self) -> FreeTrapStack {
        let ptr = exchange_scratch(self.0) as *mut TrapHandler;
        let handler = unsafe { NonNull::new_unchecked(ptr) };
        log::trace!("unload TrapStack({:#x?})", unsafe {
            handler.as_ref().range()
        });
        FreeTrapStack(handler)
    }
}

impl Drop for LoadedTrapStack {
    #[inline]
    fn drop(&mut self) {
        drop(unsafe { self.unload_unchecked() })
    }
}

/// 陷入栈内存块。
///
/// # TODO
///
/// 需要给 `Vec`、`Box<[u8]>` 之类的东西实现。
pub trait TrapStackBlock: 'static + AsRef<[u8]> + AsMut<[u8]> {}

/// 陷入处理器上下文。
#[repr(C)]
struct TrapHandler {
    /// 指向一个陷入上下文的指针。
    ///
    /// # TODO
    ///
    /// 这个东西是怎么来的？生命周期是什么？
    /// 似乎让它生命周期和陷入栈绑定也很合理。
    /// 它可以交换，只是和陷入栈同时释放而已。
    ///
    /// - 发生陷入时，将寄存器保存到此对象。
    /// - 离开陷入处理时，按此对象的内容设置寄存器。
    context: NonNull<FlowContext>,
    /// 快速路径函数。
    ///
    /// 必须在初始化陷入时设置好。
    fast_handler: FastHandler,
    /// 可在汇编使用的临时存储。
    ///
    /// - 在快速路径开始时暂存 a0。
    /// - 在快速路径结束时保存完整路径函数。
    scratch: usize,
    /// 上下文所在的内存块。
    ///
    /// 保存它以提供内存块的范围，同时用于控制内存块的生命周期。
    block: NonNull<dyn TrapStackBlock>,
    /// 禁止移动标记。
    ///
    /// `TrapHandler` 是放在其内部定义的 `block` 块里的，这是一种自引用结构，不能移动。
    pinned: PhantomPinned,
}

impl TrapHandler {
    /// 内存块地址范围。
    #[inline]
    fn range(&self) -> Range<usize> {
        let block = unsafe { self.block.as_ref().as_ref().as_ptr_range() };
        block.start as _..block.end as _
    }

    /// 如果从快速路径向完整路径转移，可以把一个对象放在栈底。
    /// 用这个方法找到栈底的一个对齐的位置。
    #[inline]
    fn locate_fast_mail<T>(&mut self) -> *mut T {
        let bottom = unsafe { self.block.as_mut() }.as_mut().as_mut_ptr();
        let offset = bottom.align_offset(align_of::<T>());
        unsafe { &mut *bottom.add(offset).cast() }
    }
}
