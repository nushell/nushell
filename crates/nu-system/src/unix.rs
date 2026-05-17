use log::warn;
use std::sync::Mutex;

/// Returns the umask for the current process
pub fn get_umask() -> u32 {
    // uucore::more::get_umask isn't threadsafe, see:
    // https://github.com/uutils/coreutils/blob/8bb31eeab9abe1c73ac5c03f3930d6e25854e4f2/src/uucore/src/lib/features/mode.rs#L172-L196
    //
    // This lock attempts to fake it.
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = match LOCK.lock() {
        Ok(g) => g,
        Err(e) => {
            warn!("umask lock poisoned. Recovering.");
            e.into_inner()
        }
    };

    uucore::mode::get_umask()
}
