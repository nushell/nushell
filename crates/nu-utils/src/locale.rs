use num_format::Locale;
use sys_locale::get_locale;

pub fn get_system_locale() -> Locale {
    let locale_string = get_locale().unwrap_or_else(|| String::from("en-US"));
    // Since get_locale() and Locale::from_name() don't always return the same items
    // we need to try and parse it to match. For instance, a valid locale is de_DE
    // however Locale::from_name() wants only de so we split and parse it out.
    let locale_string = locale_string.replace('_', "-"); // en_AU -> en-AU

    match Locale::from_name(&locale_string) {
        Ok(loc) => loc,
        _ => {
            let all = num_format::Locale::available_names();
            let locale_prefix = &locale_string.split('-').collect::<Vec<&str>>();
            if all.contains(&locale_prefix[0]) {
                // eprintln!("Found alternate: {}", &locale_prefix[0]);
                Locale::from_name(locale_prefix[0]).unwrap_or(Locale::en)
            } else {
                // eprintln!("Unable to find matching locale. Defaulting to en-US");
                Locale::en
            }
        }
    }
}