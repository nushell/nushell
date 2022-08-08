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
pub fn with_locale_override(locale_string: &str, func: fn()) {
    let result = {
        let _lock = LC_ALL_MUTEX
            .lock()
            .expect("Failed to get mutex lock for fake locale");

        let saved = std::env::var("LC_ALL").ok();
        std::env::set_var("LC_ALL", locale_string);

        let result = std::panic::catch_unwind(|| {
            func();
        });

        if let Some(locale_str) = saved {
            std::env::set_var("LC_ALL", locale_str);
        } else {
            std::env::remove_var("LC_ALL");
        }

        result
    };

    if let Err(err) = result {
        std::panic::resume_unwind(err);
    }
}
