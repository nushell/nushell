use std::fmt;

use crate::PipelineData;

/// Retains buffers for reuse in IR evaluation, avoiding heap allocation.
///
/// This is implemented in such a way that [`Clone`] is still possible, by making the fact that the
/// buffers can't be preserved on clone completely transparent. The cached buffers are always empty.
pub struct RegisterBufCache {
    bufs: Vec<Vec<PipelineData>>,
}

// SAFETY: because `bufs` only ever contains empty `Vec`s, it doesn't actually contain any of the
// data.
unsafe impl Send for RegisterBufCache {}
unsafe impl Sync for RegisterBufCache {}

impl RegisterBufCache {
    /// Create a new cache with no register buffers.
    pub const fn new() -> Self {
        RegisterBufCache { bufs: vec![] }
    }

    /// Acquire a new register buffer from the cache. The buffer will be extended to `size` with
    /// [`Empty`](PipelineData::Empty) elements.
    pub fn acquire(&mut self, size: usize) -> Vec<PipelineData> {
        let mut buf = if let Some(buf) = self.bufs.pop() {
            debug_assert!(buf.is_empty());
            buf
        } else {
            Vec::new()
        };
        buf.reserve(size);
        buf.extend(std::iter::repeat_with(|| PipelineData::Empty).take(size));
        buf
    }

    /// Release a used register buffer to the cache. The buffer will be cleared.
    pub fn release(&mut self, mut buf: Vec<PipelineData>) {
        // SAFETY: this `clear` is necessary for the `unsafe impl`s to be safe
        buf.clear();
        self.bufs.push(buf);
    }
}

impl fmt::Debug for RegisterBufCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bufs = self.bufs.len();
        let bytes: usize = self
            .bufs
            .iter()
            .map(|b| b.capacity() * std::mem::size_of::<PipelineData>())
            .sum();
        write!(f, "RegisterBufCache({bufs} bufs, {bytes} bytes)")
    }
}

impl Clone for RegisterBufCache {
    fn clone(&self) -> Self {
        RegisterBufCache::new()
    }
}
