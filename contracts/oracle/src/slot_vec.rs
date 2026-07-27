//! `SlotVec` — a capacity-reusing wrapper around [`ink::prelude::vec::Vec`].
//!
//! # Problem
//!
//! During each call to `collect_prices_batched` / `collect_prices_sequential` the
//! oracle allocates several fresh `Vec` buffers (cached sources, valid sources, price
//! results, source-update pairs).  On WASM these heap allocations dominate the
//! instruction count and contribute to gas spikes on large batches.
//!
//! # Solution
//!
//! `SlotVec<T>` wraps a `Vec<T>` and exposes a `reuse()` method that clears the
//! contents while **retaining the previously allocated capacity**.  When the same
//! `SlotVec` is reused across multiple calls (e.g. stored temporarily on the stack
//! and passed by `&mut` into helper functions) the allocator is only invoked on the
//! first call; subsequent calls simply reset the length to zero and write into the
//! existing backing buffer.
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut buf: SlotVec<PriceData> = SlotVec::with_capacity(16);
//! buf.reuse();          // clear + retain capacity
//! buf.push(item);
//! let slice = buf.as_slice();
//! ```
//!
//! The type derefs to `&[T]` / `&mut [T]` so it can be passed anywhere a slice is
//! expected without copying.

use ink::prelude::vec::Vec;

/// A `Vec<T>` wrapper that supports capacity-preserving resets.
///
/// See the [module documentation](self) for the design rationale.
pub struct SlotVec<T> {
    inner: Vec<T>,
}

impl<T> SlotVec<T> {
    /// Create a new `SlotVec` with an initial capacity hint.
    ///
    /// Choosing a capacity equal to the typical batch size avoids any
    /// allocation on the first `push` call.
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
        }
    }

    /// Clear all elements while **retaining** the backing allocation.
    ///
    /// This is the key method: calling `reuse()` before refilling the buffer
    /// means the allocator is only hit when the vector needs to grow beyond its
    /// current capacity.
    #[inline]
    pub fn reuse(&mut self) {
        self.inner.clear();
    }

    /// Append an element, growing the buffer only if needed.
    #[inline]
    pub fn push(&mut self, item: T) {
        self.inner.push(item);
    }

    /// Number of elements currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` if the buffer contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Borrow the contents as a plain slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    /// Consume the `SlotVec`, yielding the inner `Vec<T>`.
    ///
    /// Use this when you need to pass the collected data to a function that
    /// expects an owned `Vec` (e.g. returning from a message).
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.inner
    }
}

impl<T> core::ops::Deref for SlotVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for SlotVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
