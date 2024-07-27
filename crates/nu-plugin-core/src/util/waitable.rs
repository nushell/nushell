use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex, MutexGuard, PoisonError,
};

use nu_protocol::ShellError;

/// A shared container that may be empty, and allows threads to block until it has a value.
///
/// This side is read-only - use [`WaitableMut`] on threads that might write a value.
#[derive(Debug, Clone)]
pub struct Waitable<T: Clone + Send> {
    shared: Arc<WaitableShared<T>>,
}

#[derive(Debug)]
pub struct WaitableMut<T: Clone + Send> {
    shared: Arc<WaitableShared<T>>,
}

#[derive(Debug)]
struct WaitableShared<T: Clone + Send> {
    is_set: AtomicBool,
    mutex: Mutex<SyncState<T>>,
    condvar: Condvar,
}

#[derive(Debug)]
struct SyncState<T: Clone + Send> {
    writers: usize,
    value: Option<T>,
}

#[track_caller]
fn fail_if_poisoned<'a, T>(
    result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>,
) -> Result<MutexGuard<'a, T>, ShellError> {
    match result {
        Ok(guard) => Ok(guard),
        Err(_) => Err(ShellError::NushellFailedHelp {
            msg: "Waitable mutex poisoned".into(),
            help: std::panic::Location::caller().to_string(),
        }),
    }
}

impl<T: Clone + Send> WaitableMut<T> {
    /// Create a new empty `WaitableMut`. Call [`.reader()`](Self::reader) to get [`Waitable`].
    pub fn new() -> WaitableMut<T> {
        WaitableMut {
            shared: Arc::new(WaitableShared {
                is_set: AtomicBool::new(false),
                mutex: Mutex::new(SyncState {
                    writers: 1,
                    value: None,
                }),
                condvar: Condvar::new(),
            }),
        }
    }

    pub fn reader(&self) -> Waitable<T> {
        Waitable {
            shared: self.shared.clone(),
        }
    }

    /// Set the value and let waiting threads know.
    #[track_caller]
    pub fn set(&self, value: T) -> Result<(), ShellError> {
        let mut sync_state = fail_if_poisoned(self.shared.mutex.lock())?;
        self.shared.is_set.store(true, Ordering::SeqCst);
        sync_state.value = Some(value);
        self.shared.condvar.notify_all();
        Ok(())
    }
}

impl<T: Clone + Send> Default for WaitableMut<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send> Clone for WaitableMut<T> {
    fn clone(&self) -> Self {
        let shared = self.shared.clone();
        shared
            .mutex
            .lock()
            .expect("failed to lock mutex to increment writers")
            .writers += 1;
        WaitableMut { shared }
    }
}

impl<T: Clone + Send> Drop for WaitableMut<T> {
    fn drop(&mut self) {
        // Decrement writers...
        if let Ok(mut sync_state) = self.shared.mutex.lock() {
            sync_state.writers = sync_state
                .writers
                .checked_sub(1)
                .expect("would decrement writers below zero");
        }
        // and notify waiting threads so they have a chance to see it.
        self.shared.condvar.notify_all();
    }
}

impl<T: Clone + Send> Waitable<T> {
    /// Wait for a value to be available and then clone it.
    ///
    /// Returns `Ok(None)` if there are no writers left that could possibly place a value.
    #[track_caller]
    pub fn get(&self) -> Result<Option<T>, ShellError> {
        let sync_state = fail_if_poisoned(self.shared.mutex.lock())?;
        if let Some(value) = sync_state.value.clone() {
            Ok(Some(value))
        } else if sync_state.writers == 0 {
            // There can't possibly be a value written, so no point in waiting.
            Ok(None)
        } else {
            let sync_state = fail_if_poisoned(
                self.shared
                    .condvar
                    .wait_while(sync_state, |g| g.writers > 0 && g.value.is_none()),
            )?;
            Ok(sync_state.value.clone())
        }
    }

    /// Clone the value if one is available, but don't wait if not.
    #[track_caller]
    pub fn try_get(&self) -> Result<Option<T>, ShellError> {
        let sync_state = fail_if_poisoned(self.shared.mutex.lock())?;
        Ok(sync_state.value.clone())
    }

    /// Returns true if value is available.
    #[track_caller]
    pub fn is_set(&self) -> bool {
        self.shared.is_set.load(Ordering::SeqCst)
    }
}

#[test]
fn set_from_other_thread() -> Result<(), ShellError> {
    let waitable_mut = WaitableMut::new();
    let waitable = waitable_mut.reader();

    assert!(!waitable.is_set());

    std::thread::spawn(move || {
        waitable_mut.set(42).expect("error on set");
    });

    assert_eq!(Some(42), waitable.get()?);
    assert_eq!(Some(42), waitable.try_get()?);
    assert!(waitable.is_set());
    Ok(())
}

#[test]
fn dont_deadlock_if_waiting_without_writer() {
    use std::time::Duration;

    let (tx, rx) = std::sync::mpsc::channel();
    let writer = WaitableMut::<()>::new();
    let waitable = writer.reader();
    // Ensure there are no writers
    drop(writer);
    std::thread::spawn(move || {
        let _ = tx.send(waitable.get());
    });
    let result = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("timed out")
        .expect("error");
    assert!(result.is_none());
}
