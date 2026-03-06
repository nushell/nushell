use std::{
    collections::HashMap,
    hash::Hash,
    pin::Pin,
    sync::{LazyLock, OnceLock},
};

use parking_lot::RwLock;

/// Lazily initializes values per key.
///
/// The first call to [`KeyedLazyLock::get`] for a key creates the value using `init`.
/// Later calls return the same value.
///
/// Initialization for each key happens at most once.
pub struct KeyedLazyLock<K, V, F> {
    // Why `Box<OnceLock<V>>`
    //
    // Each key stores its own OnceLock. We allocate it in a Box so the address
    // stays stable even if the HashMap grows and relocates entries.
    //
    // This lets us:
    // 1. Grab a raw pointer to the OnceLock.
    // 2. Drop the map lock.
    // 3. Initialize the value outside the lock.
    //
    // Without the Box, the OnceLock could move during a HashMap resize,
    // invalidating the pointer.
    map: LazyLock<RwLock<HashMap<K, Pin<Box<OnceLock<V>>>>>>,
    init: F,
}

impl<K, V, F> KeyedLazyLock<K, V, F> {
    pub const fn new(init: F) -> Self {
        Self {
            map: LazyLock::new(|| RwLock::new(HashMap::new())),
            init,
        }
    }
}

impl<K, V, F> KeyedLazyLock<K, V, F>
where
    K: Eq + Hash + Clone,
    F: Fn(&K) -> V,
{
    /// Returns the lazily initialized value for `key`.
    ///
    /// If the key has not been accessed before, `init(key)` will run exactly once.
    /// Concurrent callers requesting the same key will wait for initialization.
    ///
    /// # Deadlocks
    /// `init` must not call `get` with the same key.
    pub fn get(&self, key: &K) -> &V {
        // Fast path: try to find the cell with a read lock.
        if let Some(cell_ptr) = self.try_get_cell_ptr(key) {
            // SAFETY:
            // - The pointer refers to a OnceLock stored inside a Box in the map.
            // - Entries are never removed, so the Box lives until self is dropped.
            // - Moving the Box inside the HashMap does not move the allocation.
            let cell = unsafe { &*cell_ptr };

            // init runs outside the map lock.
            return cell.get_or_init(|| (self.init)(key));
        }

        // Slow path: insert the cell.
        let cell_ptr = {
            let mut write = self.map.write();

            // Another thread may have inserted it already.
            let cell_box = write
                .entry(key.clone())
                .or_insert_with(|| Box::pin(OnceLock::new()));

            // Grab pointer so we can drop the lock before initialization.
            (&**cell_box) as *const OnceLock<V>
        };

        // SAFETY: same reasoning as above.
        let cell = unsafe { &*cell_ptr };
        cell.get_or_init(|| (self.init)(key))
    }

    #[inline]
    fn try_get_cell_ptr(&self, key: &K) -> Option<*const OnceLock<V>> {
        let read = self.map.read();
        read.get(key).map(|cell_box| (&**cell_box) as *const OnceLock<V>)
    }
}