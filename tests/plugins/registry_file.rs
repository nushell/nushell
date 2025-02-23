use std::{fs::File, path::PathBuf};

use nu_protocol::{PluginRegistryFile, PluginRegistryItem, PluginRegistryItemData};
use nu_test_support::{fs::Stub, nu, nu_with_plugins, playground::Playground};

fn example_plugin_path() -> PathBuf {
    nu_test_support::commands::ensure_plugins_built();

    let bins_path = nu_test_support::fs::binaries();
    nu_path::canonicalize_with(
        if cfg!(windows) {
            "nu_plugin_example.exe"
        } else {
            "nu_plugin_example"
        },
        bins_path,
    )
    .expect("nu_plugin_example not found")
}

fn valid_plugin_item_data() -> PluginRegistryItemData {
    PluginRegistryItemData::Valid {
        metadata: Default::default(),
        commands: vec![],
    }
}

#[test]
fn plugin_add_then_restart_nu() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!("
            plugin add '{}'
            (
                ^$nu.current-exe
                    --config $nu.config-path
                    --env-config $nu.env-path
                    --plugin-config $nu.plugin-path
                    --commands 'plugin list --engine | get name | to json --raw'
            )
        ", example_plugin_path().display())
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_add_in_nu_plugin_dirs_const() {
    let example_plugin_path = example_plugin_path();

    let dirname = example_plugin_path.parent().expect("no parent");
    let filename = example_plugin_path
        .file_name()
        .expect("no file_name")
        .to_str()
        .expect("not utf-8");

    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(
            r#"
                $env.NU_PLUGIN_DIRS = null
                const NU_PLUGIN_DIRS = ['{0}']
                plugin add '{1}'
                (
                    ^$nu.current-exe
                        --config $nu.config-path
                        --env-config $nu.env-path
                        --plugin-config $nu.plugin-path
                        --commands 'plugin list --engine | get name | to json --raw'
                )
            "#,
            dirname.display(),
            filename
        )
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_add_in_nu_plugin_dirs_env() {
    let example_plugin_path = example_plugin_path();

    let dirname = example_plugin_path.parent().expect("no parent");
    let filename = example_plugin_path
        .file_name()
        .expect("no file_name")
        .to_str()
        .expect("not utf-8");

    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(
            r#"
                $env.NU_PLUGIN_DIRS = ['{0}']
                plugin add '{1}'
                (
                    ^$nu.current-exe
                        --config $nu.config-path
                        --env-config $nu.env-path
                        --plugin-config $nu.plugin-path
                        --commands 'plugin list --engine | get name | to json --raw'
                )
            "#,
            dirname.display(),
            filename
        )
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_add_to_custom_path() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("plugin add to custom path", |dirs, _playground| {
        let result = nu!(
            cwd: dirs.test(),
            &format!("
                plugin add --plugin-config test-plugin-file.msgpackz '{}'
            ", example_plugin_path.display())
        );

        assert!(result.status.success());

        let contents = PluginRegistryFile::read_from(
            File::open(dirs.test().join("test-plugin-file.msgpackz"))
                .expect("failed to open plugin file"),
            None,
        )
        .expect("failed to read plugin file");

        assert_eq!(1, contents.plugins.len());
        assert_eq!("example", contents.plugins[0].name);
    })
}

#[test]
fn plugin_rm_then_restart_nu() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("plugin rm from custom path", |dirs, playground| {
        playground.with_files(&[
            Stub::FileWithContent("config.nu", ""),
            Stub::FileWithContent("env.nu", ""),
        ]);

        let file = File::create(dirs.test().join("test-plugin-file.msgpackz"))
            .expect("failed to create file");
        let mut contents = PluginRegistryFile::new();

        contents.upsert_plugin(PluginRegistryItem {
            name: "example".into(),
            filename: example_plugin_path,
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents.upsert_plugin(PluginRegistryItem {
            name: "foo".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_foo").into(),
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents
            .write_to(file, None)
            .expect("failed to write plugin file");

        assert_cmd::Command::new(nu_test_support::fs::executable_path())
            .current_dir(dirs.test())
            .args([
                "--no-std-lib",
                "--config",
                "config.nu",
                "--env-config",
                "env.nu",
                "--plugin-config",
                "test-plugin-file.msgpackz",
                "--commands",
                "plugin rm example",
            ])
            .assert()
            .success()
            .stderr("");

        assert_cmd::Command::new(nu_test_support::fs::executable_path())
            .current_dir(dirs.test())
            .args([
                "--no-std-lib",
                "--config",
                "config.nu",
                "--env-config",
                "env.nu",
                "--plugin-config",
                "test-plugin-file.msgpackz",
                "--commands",
                "plugin list --engine | get name | to json --raw",
            ])
            .assert()
            .success()
            .stdout("[\"foo\"]\n");
    })
}

#[test]
fn plugin_rm_not_found() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        r#"
            plugin rm example
        "#
    );
    assert!(!result.status.success());
    assert!(result.err.contains("example"));
}

#[test]
fn plugin_rm_from_custom_path() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("plugin rm from custom path", |dirs, _playground| {
        let file = File::create(dirs.test().join("test-plugin-file.msgpackz"))
            .expect("failed to create file");
        let mut contents = PluginRegistryFile::new();

        contents.upsert_plugin(PluginRegistryItem {
            name: "example".into(),
            filename: example_plugin_path,
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents.upsert_plugin(PluginRegistryItem {
            name: "foo".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_foo").into(),
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents
            .write_to(file, None)
            .expect("failed to write plugin file");

        let result = nu!(
            cwd: dirs.test(),
            "plugin rm --plugin-config test-plugin-file.msgpackz example",
        );
        assert!(result.status.success());
        assert!(result.err.trim().is_empty());

        // Check the contents after running
        let contents = PluginRegistryFile::read_from(
            File::open(dirs.test().join("test-plugin-file.msgpackz")).expect("failed to open file"),
            None,
        )
        .expect("failed to read file");

        assert!(!contents.plugins.iter().any(|p| p.name == "example"));

        // Shouldn't remove anything else
        assert!(contents.plugins.iter().any(|p| p.name == "foo"));
    })
}

#[test]
fn plugin_rm_using_filename() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("plugin rm using filename", |dirs, _playground| {
        let file = File::create(dirs.test().join("test-plugin-file.msgpackz"))
            .expect("failed to create file");
        let mut contents = PluginRegistryFile::new();

        contents.upsert_plugin(PluginRegistryItem {
            name: "example".into(),
            filename: example_plugin_path.clone(),
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents.upsert_plugin(PluginRegistryItem {
            name: "foo".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_foo").into(),
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents
            .write_to(file, None)
            .expect("failed to write plugin file");

        let result = nu!(
            cwd: dirs.test(),
            &format!(
                "plugin rm --plugin-config test-plugin-file.msgpackz '{}'",
                example_plugin_path.display()
            )
        );
        assert!(result.status.success());
        assert!(result.err.trim().is_empty());

        // Check the contents after running
        let contents = PluginRegistryFile::read_from(
            File::open(dirs.test().join("test-plugin-file.msgpackz")).expect("failed to open file"),
            None,
        )
        .expect("failed to read file");

        assert!(!contents.plugins.iter().any(|p| p.name == "example"));

        // Shouldn't remove anything else
        assert!(contents.plugins.iter().any(|p| p.name == "foo"));
    })
}

/// Running nu with a test plugin file that fails to parse on one plugin should just cause a warning
/// but the others should be loaded
#[test]
fn warning_on_invalid_plugin_item() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("warning on invalid plugin item", |dirs, playground| {
        playground.with_files(&[
            Stub::FileWithContent("config.nu", ""),
            Stub::FileWithContent("env.nu", ""),
        ]);

        let file = File::create(dirs.test().join("test-plugin-file.msgpackz"))
            .expect("failed to create file");
        let mut contents = PluginRegistryFile::new();

        contents.upsert_plugin(PluginRegistryItem {
            name: "example".into(),
            filename: example_plugin_path,
            shell: None,
            data: valid_plugin_item_data(),
        });

        contents.upsert_plugin(PluginRegistryItem {
            name: "badtest".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_badtest").into(),
            shell: None,
            data: PluginRegistryItemData::Invalid,
        });

        contents
            .write_to(file, None)
            .expect("failed to write plugin file");

        let result = assert_cmd::Command::new(nu_test_support::fs::executable_path())
            .current_dir(dirs.test())
            .args([
                "--no-std-lib",
                "--config",
                "config.nu",
                "--env-config",
                "env.nu",
                "--plugin-config",
                "test-plugin-file.msgpackz",
                "--commands",
                "plugin list --engine | get name | to json --raw",
            ])
            .output()
            .expect("failed to run nu");

        let out = String::from_utf8_lossy(&result.stdout).trim().to_owned();
        let err = String::from_utf8_lossy(&result.stderr).trim().to_owned();

        println!("=== stdout\n{out}\n=== stderr\n{err}");

        // The code should still execute successfully
        assert!(result.status.success());
        // The "example" plugin should be unaffected
        assert_eq!(r#"["example"]"#, out);
        // The warning should be in there
        assert!(err.contains("registered plugin data"));
        assert!(err.contains("badtest"));
    })
}

#[test]
fn plugin_use_error_not_found() {
    Playground::setup("plugin use error not found", |dirs, playground| {
        playground.with_files(&[
            Stub::FileWithContent("config.nu", ""),
            Stub::FileWithContent("env.nu", ""),
        ]);

        // Make an empty msgpackz
        let file = File::create(dirs.test().join("plugin.msgpackz"))
            .expect("failed to open plugin.msgpackz");
        PluginRegistryFile::default()
            .write_to(file, None)
            .expect("failed to write empty registry file");

        let output = assert_cmd::Command::new(nu_test_support::fs::executable_path())
            .current_dir(dirs.test())
            .args(["--config", "config.nu"])
            .args(["--env-config", "env.nu"])
            .args(["--plugin-config", "plugin.msgpackz"])
            .args(["--commands", "plugin use custom_values"])
            .output()
            .expect("failed to run nu");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Plugin not found"));
    })
}

#[test]
fn plugin_shows_up_in_default_plugin_list_after_add() {
    let example_plugin_path = example_plugin_path();
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(r#"
            plugin add '{}'
            plugin list | get status | to json --raw
        "#, example_plugin_path.display())
    );
    assert!(result.status.success());
    assert_eq!(r#"["added"]"#, result.out);
}

#[test]
fn plugin_shows_removed_after_removing() {
    let example_plugin_path = example_plugin_path();
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(r#"
            plugin add '{}'
            plugin list | get status | to json --raw
            (
                ^$nu.current-exe
                    --config $nu.config-path
                    --env-config $nu.env-path
                    --plugin-config $nu.plugin-path
                    --commands 'plugin rm example; plugin list | get status | to json --raw'
            )
        "#, example_plugin_path.display())
    );
    assert!(result.status.success());
    assert_eq!(r#"["removed"]"#, result.out);
}

#[test]
fn plugin_add_and_then_use() {
    let example_plugin_path = example_plugin_path();
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(r#"
            plugin add '{}'
            (
                ^$nu.current-exe
                    --config $nu.config-path
                    --env-config $nu.env-path
                    --plugin-config $nu.plugin-path
                    --commands 'plugin use example; plugin list --engine | get name | to json --raw'
            )
        "#, example_plugin_path.display())
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_add_and_then_use_by_filename() {
    let example_plugin_path = example_plugin_path();
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!(r#"
            plugin add '{0}'
            (
                ^$nu.current-exe
                    --config $nu.config-path
                    --env-config $nu.env-path
                    --plugin-config $nu.plugin-path
                    --commands 'plugin use '{0}'; plugin list --engine | get name | to json --raw'
            )
        "#, example_plugin_path.display())
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_add_then_use_with_custom_path() {
    let example_plugin_path = example_plugin_path();
    Playground::setup("plugin add to custom path", |dirs, _playground| {
        let result_add = nu!(
            cwd: dirs.test(),
            &format!("
                plugin add --plugin-config test-plugin-file.msgpackz '{}'
            ", example_plugin_path.display())
        );

        assert!(result_add.status.success());

        let result_use = nu!(
            cwd: dirs.test(),
            r#"
                plugin use --plugin-config test-plugin-file.msgpackz example
                plugin list --engine | get name | to json --raw
            "#
        );

        assert!(result_use.status.success());
        assert_eq!(r#"["example"]"#, result_use.out);
    })
}
