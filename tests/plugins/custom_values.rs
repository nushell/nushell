use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_custom_value_from_plugin_and_instantly_collapse_it() -> Result {
    test()
        .run("custom-value generate | into value")
        .expect_value_eq("I used to be a custom value! My data was (abc)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_custom_value_from_plugin_and_pass_it_over() -> Result {
    test()
        .run("custom-value generate | custom-value update | into value")
        .expect_value_eq("I used to be a custom value! My data was (abcxyz)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_custom_value_from_plugin_and_pass_it_over_as_an_argument() -> Result {
    test()
        .run("custom-value update-arg (custom-value generate) | into value")
        .expect_value_eq("I used to be a custom value! My data was (abcxyz)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_generate_and_update_multiple_types_of_custom_values() -> Result {
    test()
        .run("custom-value generate2 | custom-value update | into value")
        .expect_value_eq("I used to be a DIFFERENT custom value! (xyzabc)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_generate_custom_value_and_pass_through_closure() -> Result {
    test()
        .run("custom-value generate2 { custom-value update } | into value")
        .expect_value_eq("I used to be a DIFFERENT custom value! (xyzabc)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_describe_plugin_custom_values() -> Result {
    test()
        .run("custom-value generate | describe")
        .expect_value_eq("CoolCustomValue")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_plugin_custom_value_int_cell_path() -> Result {
    test()
        .run("(custom-value generate).0")
        .expect_value_eq("abc")?;

    test()
        .run("(custom-value generate).1")
        .expect_error_code("nu::shell::access_beyond_end")?;

    test()
        .run("(custom-value generate).1?")
        .expect_value_eq(())?;

    Ok(())
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_get_plugin_custom_value_string_cell_path() -> Result {
    test()
        .run("(custom-value generate).cool")
        .expect_value_eq("abc")?;

    test()
        .run("(custom-value generate).meh")
        .expect_error_code("nu::shell::column_not_found")?;

    test()
        .run("(custom-value generate).meh?")
        .expect_value_eq(())?;

    test()
        .run("(custom-value generate).COOL")
        .expect_error_code("nu::shell::column_not_found")?;

    test()
        .run("(custom-value generate).COOL!")
        .expect_value_eq("abc")?;

    Ok(())
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_sort_plugin_custom_values() -> Result {
    let code = "
        [(custom-value generate | custom-value update), (custom-value generate)]
        | sort 
        | each { into value }
    ";

    test().run(code).expect_value_eq([
        "I used to be a custom value! My data was (abc)",
        "I used to be a custom value! My data was (abcxyz)",
    ])
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn can_append_plugin_custom_values() -> Result {
    test()
        .run("(custom-value generate) ++ (custom-value generate) | into value")
        .expect_value_eq("I used to be a custom value! My data was (abcabc)")
}

// There are currently no custom values defined by the engine that aren't hidden behind an extra
// feature
#[cfg(feature = "sqlite")]
#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn fails_if_passing_engine_custom_values_to_plugins() -> Result {
    let err = test()
        .cwd("tests/fixtures/formats")
        .run("open sample.db | custom-value update")
        .expect_shell_error()?;
    let err = err.to_string();
    assert_contains("`SQLiteDatabase` cannot be sent to plugin", err);
    Ok(())
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES, NU_PLUGIN_INC)]
fn fails_if_passing_custom_values_across_plugins() -> Result {
    let err = test()
        .run("custom-value generate | inc --major")
        .expect_shell_error()?;
    let err = err.to_string();
    assert_contains("`CoolCustomValue` cannot be sent to plugin", err);
    Ok(())
}

#[test]
#[deps(NU, NU_PLUGIN_CUSTOM_VALUES)]
fn drop_check_custom_value_prints_message_on_drop() -> Result {
    // We build an array with the value copied twice to verify that it only gets dropped once
    let commands = "do {{|v| [$v $v]}} (custom-value drop-check 'Hello') | ignore";
    let code = format!(
        "nu -n --plugins {} -c $in | complete | get stderr",
        NU_PLUGIN_CUSTOM_VALUES.path().to_string_lossy()
    );

    test()
        .run_with_data(code, commands)
        .expect_value_eq("DropCheckValue was dropped: Hello\n")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn handle_make_then_get_success() -> Result {
    // The drop notification must wait until the `handle get` call has finished in order for this
    // to succeed
    test()
        .run("42 | custom-value handle make | custom-value handle get")
        .expect_value_eq(42)
}

#[test]
#[serial]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn handle_update_several_times_doesnt_deadlock() -> Result {
    let code = r#"
        "hEllO" |
            custom-value handle make |
            custom-value handle update { str upcase } |
            custom-value handle update { str downcase } |
            custom-value handle update { str title-case } |
            custom-value handle get
    "#;

    // Do this in a loop to try to provoke a deadlock on drop
    for _ in 0..10 {
        test().run(code).expect_value_eq("Hello")?;
    }

    Ok(())
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn custom_value_in_example_is_rendered() -> Result {
    let out: String = test().run("custom-value generate --help")?;
    assert_contains("I used to be a custom value! My data was (abc)", out);
    Ok(())
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn custom_value_into_string() -> Result {
    test()
        .run("custom-value generate | into string")
        .expect_value_eq("I used to be a custom value! My data was (abc)")
}

#[test]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn save_custom_values() -> Result {
    Playground::setup("save custom values", |dirs, _| {
        let unimplemented = test()
            .cwd(dirs.test())
            .run("custom-value generate | save file")
            .expect_shell_error()?;
        assert_contains(
            "Cannot save custom value",
            unimplemented.to_string(),
        );

        let () = test()
            .cwd(dirs.test())
            .run("custom-value generate2 | save file")?;
        let content = std::fs::read_to_string(dirs.test().join("file")).unwrap();
        assert_eq!(content, "xyz"); // "xyz" is the content when using generate2

        Ok(())
    })
}
