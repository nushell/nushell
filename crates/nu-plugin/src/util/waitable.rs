use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex, MutexGuard, PoisonError,
};

use nu_protocol::ShellError;

/// A container that may be empty, and allows threads to block until it has a value.
#[derive(Debug)]
pub struct Waitable<T: Clone + Send> {
    is_set: AtomicBool,
    mutex: Mutex<Option<T>>,
    condvar: Condvar,
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

impl<T: Clone + Send> Waitable<T> {
    /// Create a new empty `Waitable`.
    pub fn new() -> Waitable<T> {
        Waitable {
            is_set: AtomicBool::new(false),
            mutex: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }

    /// Wait for a value to be available and then clone it.
    #[track_caller]
    pub fn get(&self) -> Result<T, ShellError> {
        let guard = fail_if_poisoned(self.mutex.lock())?;
        if let Some(value) = (*guard).clone() {
            Ok(value)
        } else {
            let guard = fail_if_poisoned(self.condvar.wait_while(guard, |g| g.is_none()))?;
            Ok((*guard)
                .clone()
                .expect("checked already for Some but it was None"))
        }
    }

    /// Clone the value if one is available, but don't wait if not.
    #[track_caller]
    pub fn try_get(&self) -> Result<Option<T>, ShellError> {
        let guard = fail_if_poisoned(self.mutex.lock())?;
        Ok((*guard).clone())
    }

    /// Returns true if value is available.
    #[track_caller]
    pub fn is_set(&self) -> bool {
        self.is_set.load(Ordering::SeqCst)
    }

    /// Set the value and let waiting threads know.
    #[track_caller]
    pub fn set(&self, value: T) -> Result<(), ShellError> {
        let mut guard = fail_if_poisoned(self.mutex.lock())?;
        self.is_set.store(true, Ordering::SeqCst);
        *guard = Some(value);
        self.condvar.notify_all();
        Ok(())
    }
}

impl<T: Clone + Send> Default for Waitable<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn set_from_other_thread() -> Result<(), ShellError> {
    use std::sync::Arc;

    let waitable = Arc::new(Waitable::new());
    let waitable_clone = waitable.clone();

    assert!(!waitable.is_set());

    std::thread::spawn(move || {
        waitable_clone.set(42).expect("error on set");
    });

    assert_eq!(42, waitable.get()?);
    assert_eq!(Some(42), waitable.try_get()?);
    assert!(waitable.is_set());
    Ok(())
}
