use nu_test_support::nu_with_plugins;
use pretty_assertions::assert_eq;

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
fn can_get_custom_value_from_plugin_and_pass_it_over_as_an_argument() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value update-arg (custom-value generate)"
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
fn can_generate_custom_value_and_pass_through_closure() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate2 { custom-value update }"
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

#[test]
fn can_get_plugin_custom_value_int_cell_path() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).0"
    );

    assert_eq!(actual.out, "abc");
}

#[test]
fn can_get_plugin_custom_value_string_cell_path() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).cool"
    );

    assert_eq!(actual.out, "abc");
}

#[test]
fn can_sort_plugin_custom_values() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "[(custom-value generate | custom-value update) (custom-value generate)] | sort | each { print } | ignore"
    );

    assert_eq!(
        actual.out,
        "I used to be a custom value! My data was (abc)\
        I used to be a custom value! My data was (abcxyz)"
    );
}

#[test]
fn can_append_plugin_custom_values() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate) ++ (custom-value generate)"
    );

    assert_eq!(
        actual.out,
        "I used to be a custom value! My data was (abcabc)"
    );
}

// There are currently no custom values defined by the engine that aren't hidden behind an extra
// feature
#[cfg(feature = "sqlite")]
#[test]
fn fails_if_passing_engine_custom_values_to_plugins() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_custom_values"),
        "open sample.db | custom-value update"
    );

    assert!(actual
        .err
        .contains("`SQLiteDatabase` cannot be sent to plugin"));
    assert!(actual
        .err
        .contains("the `custom_values` plugin does not support this kind of value"));
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
        .contains("`CoolCustomValue` cannot be sent to plugin"));
    assert!(actual
        .err
        .contains("the `inc` plugin does not support this kind of value"));
}

#[test]
fn drop_check_custom_value_prints_message_on_drop() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        // We build an array with the value copied twice to verify that it only gets dropped once
        "do { |v| [$v $v] } (custom-value drop-check 'Hello') | ignore"
    );

    assert_eq!(actual.err, "DropCheck was dropped: Hello\n");
    assert!(actual.status.success());
}
