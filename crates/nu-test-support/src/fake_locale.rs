use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    static ref LC_ALL_MUTEX: Mutex<()> = Mutex::new(());
}

/// Run a closure in a fake locale environment.
///
/// Before the closure is executed, the environmen variable `LC_ALL` is set to
/// the value provided by `locale_string`. When the closure is done, the
/// original `LC_ALL` value is restored.
///
/// Environment variables are global values. So when they are changed by one
/// thread they are changed for all others. To prevent a test from overwriting
/// the `LC_ALL` environment variable of another test, a mutex is used.
pub fn with_fake_locale(locale_string: &str, func: fn()) {
    let _lock = LC_ALL_MUTEX.lock().unwrap();

    let saved = std::env::var("LC_ALL").ok();
    std::env::set_var("LC_ALL", locale_string);

    func();

    if let Some(locale_str) = saved {
        std::env::set_var("LC_ALL", locale_str);
    } else {
        std::env::remove_var("LC_ALL");
    }
}
