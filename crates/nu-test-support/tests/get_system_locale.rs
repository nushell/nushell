#![cfg(debug_assertions)]

use nu_protocol::Locale;
use nu_test_support::locale_override::with_locale_override;

#[test]
fn test_get_system_locale_en() {
    let locale = with_locale_override("en_US.UTF-8", Locale::system_number).unwrap();
    assert_eq!(locale.name(), "en");
}

#[test]
fn test_get_system_locale_de() {
    let locale = with_locale_override("de_DE.UTF-8", Locale::system_number).unwrap();
    assert_eq!(locale.name(), "de");
}
