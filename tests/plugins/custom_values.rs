use nu_test_support::nu_with_plugins;

#[test]
fn can_get_custom_value_from_plugin_and_instantly_collapse_it() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate"
    );

    assert_eq!(actual.out, "I used to be a custom value! My data was (abc)");
}

#[test]
fn can_get_custom_value_from_plugin_and_pass_it_over() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate | custom-value update"
    );

    assert_eq!(
        actual.out,
        "I used to be a custom value! My data was (abcxyz)"
    );
}

#[test]
fn can_generate_and_updated_multiple_types_of_custom_values() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate2 | custom-value update"
    );

    assert_eq!(
        actual.out,
        "I used to be a DIFFERENT custom value! (xyzabc)"
    );
}

#[test]
fn can_get_describe_plugin_custom_values() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate | describe"
    );

    assert_eq!(actual.out, "CoolCustomValue");
}

// There are currently no custom values defined by the engine that aren't hidden behind an extra
// feature, both database and dataframes are hidden behind --features=extra so we need to guard
// this test
#[cfg(feature = "database")]
#[test]
fn fails_if_passing_engine_custom_values_to_plugins() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_custom_values"),
        "open sample.db | custom-value update"
    );

    assert!(actual
        .err
        .contains("Plugin custom-value update can not handle the custom value SQLiteDatabase"));
}

#[test]
fn fails_if_passing_custom_values_across_plugins() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugins: [
            ("nu_plugin_custom_values"),
            ("nu_plugin_inc")
        ],
        "custom-value generate | inc --major"
    );

    assert!(actual
        .err
        .contains("Plugin inc can not handle the custom value CoolCustomValue"));
}
