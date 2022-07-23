use nu_test_support::nu_with_plugins;

#[test]
fn can_get_custom_value_from_plugin_and_instantly_collapse_it() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("json", "nu_plugin_custom_values"),
        "custom-value generate"
    );

    assert_eq!(actual.out, "I used to be a custom value! My data was (abc)");
}

#[test]
fn can_get_custom_value_from_plugin_and_pass_it_over() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("json", "nu_plugin_custom_values"),
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
        plugin: ("json", "nu_plugin_custom_values"),
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
        plugin: ("json", "nu_plugin_custom_values"),
        "custom-value generate | describe"
    );

    assert_eq!(actual.out, "CoolCustomValue");
}
