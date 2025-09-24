use nu_test_support::{nu_with_plugins, playground::Playground};
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
    let zero_index = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).0"
    );
    assert_eq!(zero_index.out, "abc");

    let one_index = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).1"
    );
    assert!(one_index.err.contains("nu::shell::access_beyond_end"));

    let one_index_optional = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).1? | describe"
    );
    assert_eq!(one_index_optional.out, "nothing");
}

#[test]
fn can_get_plugin_custom_value_string_cell_path() {
    let cool = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).cool"
    );
    assert_eq!(cool.out, "abc");

    let meh = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).meh"
    );
    assert!(meh.err.contains("nu::shell::column_not_found"));

    let meh_optional = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).meh? | describe"
    );
    assert_eq!(meh_optional.out, "nothing");

    let cool_capitalized_sensitive = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).COOL"
    );
    assert!(
        cool_capitalized_sensitive
            .err
            .contains("nu::shell::column_not_found")
    );

    let cool_capitalized_insensitive = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "(custom-value generate).COOL!"
    );
    assert_eq!(cool_capitalized_insensitive.out, "abc");
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

    assert!(
        actual
            .err
            .contains("`SQLiteDatabase` cannot be sent to plugin")
    );
    assert!(
        actual
            .err
            .contains("the `custom_values` plugin does not support this kind of value")
    );
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

    assert!(
        actual
            .err
            .contains("`CoolCustomValue` cannot be sent to plugin")
    );
    assert!(
        actual
            .err
            .contains("the `inc` plugin does not support this kind of value")
    );
}

#[test]
fn drop_check_custom_value_prints_message_on_drop() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        // We build an array with the value copied twice to verify that it only gets dropped once
        "do { |v| [$v $v] } (custom-value drop-check 'Hello') | ignore"
    );

    assert_eq!(actual.err, "DropCheckValue was dropped: Hello\n");
    assert!(actual.status.success());
}

#[test]
fn handle_make_then_get_success() {
    // The drop notification must wait until the `handle get` call has finished in order for this
    // to succeed
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "42 | custom-value handle make | custom-value handle get"
    );

    assert_eq!(actual.out, "42");
    assert!(actual.status.success());
}

#[test]
fn handle_update_several_times_doesnt_deadlock() {
    // Do this in a loop to try to provoke a deadlock on drop
    for _ in 0..10 {
        let actual = nu_with_plugins!(
            cwd: "tests",
            plugin: ("nu_plugin_custom_values"),
            r#"
                "hEllO" |
                    custom-value handle make |
                    custom-value handle update { str upcase } |
                    custom-value handle update { str downcase } |
                    custom-value handle update { str title-case } |
                    custom-value handle get
            "#
        );

        assert_eq!(actual.out, "Hello");
        assert!(actual.status.success());
    }
}

#[test]
fn custom_value_in_example_is_rendered() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate --help"
    );

    assert!(
        actual
            .out
            .contains("I used to be a custom value! My data was (abc)")
    );
    assert!(actual.status.success());
}

#[test]
fn custom_value_into_string() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_custom_values"),
        "custom-value generate | into string"
    );

    assert_eq!(actual.out, "I used to be a custom value! My data was (abc)");
}

#[test]
fn save_custom_values() {
    Playground::setup("save custom values", |_, playground| {
        let actual_unimplemented = nu_with_plugins!(
            cwd: playground.cwd(),
            plugin: ("nu_plugin_custom_values"),
            "custom-value generate | save file"
        );
        assert!(
            actual_unimplemented
                .err
                .contains("Custom value does not implement `save`")
        );

        nu_with_plugins!(
            cwd: playground.cwd(),
            plugin: ("nu_plugin_custom_values"),
            "custom-value generate2 | save file"
        );

        let file_path = playground.cwd().join("file");
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "xyz"); // "xyz" is the content when using generate2
    });
}
