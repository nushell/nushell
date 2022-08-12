#![cfg(debug_assertions)]

use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use nu_utils::locale::LOCALE_OVERRIDE_ENV_VAR;

lazy_static! {
    static ref LOCALE_OVERRIDE_MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

/// Run a closure in a fake locale environment.
///
/// Before the closure is executed, an environment variable whose name is
/// defined in `nu_utils::locale::LOCALE_OVERRIDE_ENV_VAR` is set to the value
/// provided by `locale_string`. When the closure is done, the previous value is
/// restored.
///
/// Environment variables are global values. So when they are changed by one
/// thread they are changed for all others. To prevent a test from overwriting
/// the environment variable of another test, a mutex is used.
pub fn with_locale_override<T>(locale_string: &str, func: fn() -> T) -> T {
    let result = {
        let _lock = LOCALE_OVERRIDE_MUTEX
            .lock()
            .expect("Failed to get mutex lock for locale override");

        let saved = std::env::var(LOCALE_OVERRIDE_ENV_VAR).ok();
        std::env::set_var(LOCALE_OVERRIDE_ENV_VAR, locale_string);

        let result = std::panic::catch_unwind(func);

        if let Some(locale_str) = saved {
            std::env::set_var(LOCALE_OVERRIDE_ENV_VAR, locale_str);
        } else {
            std::env::remove_var(LOCALE_OVERRIDE_ENV_VAR);
        }

        result
    };

    match result {
        Ok(result) => result,
        Err(err) => std::panic::resume_unwind(err),
    }
}
