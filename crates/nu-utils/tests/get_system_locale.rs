use nu_test_support::fake_locale::with_fake_locale;
use nu_utils::get_system_locale;
use num_format::Grouping;

#[test]
fn test_get_system_locale_en() {
    with_fake_locale("en_US.UTF-8", || {
        let locale = get_system_locale();

        assert_eq!(locale.name(), "en");
        assert_eq!(locale.grouping(), Grouping::Standard)
    })
}

#[test]
fn test_get_system_locale_de() {
    with_fake_locale("de_DE.UTF-8", || {
        let locale = get_system_locale();

        assert_eq!(locale.name(), "de");
        assert_eq!(locale.grouping(), Grouping::Standard)
    });
}
