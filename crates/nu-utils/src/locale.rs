use num_format::Locale;

pub const LOCALE_OVERRIDE_ENV_VAR: &str = "NU_TEST_LOCALE_OVERRIDE";

pub fn get_system_locale() -> Locale {
    let locale_string = get_system_locale_string().unwrap_or_else(|| String::from("en-US"));
    // Since get_locale() and Locale::from_name() don't always return the same items
    // we need to try and parse it to match. For instance, a valid locale is de_DE
    // however Locale::from_name() wants only de so we split and parse it out.
    let locale_string = locale_string.replace('_', "-"); // en_AU -> en-AU

    Locale::from_name(&locale_string).unwrap_or_else(|_| {
        let all = num_format::Locale::available_names();
        let locale_prefix = &locale_string.split('-').collect::<Vec<&str>>();
        if all.contains(&locale_prefix[0]) {
            Locale::from_name(locale_prefix[0]).unwrap_or(Locale::en)
        } else {
            Locale::en
        }
    })
}

#[cfg(debug_assertions)]
pub fn get_system_locale_string() -> Option<String> {
    std::env::var(LOCALE_OVERRIDE_ENV_VAR).ok().or_else(
        #[cfg(not(test))]
        {
            sys_locale::get_locale
        },
        #[cfg(test)]
        {
            // For tests, we use the same locale on all systems.
            // To override this, set `LOCALE_OVERRIDE_ENV_VAR`.
            || Some(Locale::en_US_POSIX.name().to_owned())
        },
    )
}

#[cfg(not(debug_assertions))]
pub fn get_system_locale_string() -> Option<String> {
    sys_locale::get_locale()
}
