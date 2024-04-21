use std::{
    fs::File,
    path::PathBuf,
    process::{Command, Stdio},
};

use nu_protocol::{PluginCacheFile, PluginCacheItem, PluginCacheItemData};
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
                    --commands 'plugin list | get name | to json --raw'
            )
        ", example_plugin_path().display())
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

        let contents = PluginCacheFile::read_from(
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
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            plugin rm example
            ^$nu.current-exe --config $nu.config-path --env-config $nu.env-path --plugin-config $nu.plugin-path --commands 'plugin list | get name | to json --raw'
        "#
    );
    assert!(result.status.success());
    assert_eq!(r#"[]"#, result.out);
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
        let mut contents = PluginCacheFile::new();

        contents.upsert_plugin(PluginCacheItem {
            name: "example".into(),
            filename: example_plugin_path,
            shell: None,
            data: PluginCacheItemData::Valid { commands: vec![] },
        });

        contents.upsert_plugin(PluginCacheItem {
            name: "foo".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_foo"),
            shell: None,
            data: PluginCacheItemData::Valid { commands: vec![] },
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
        let contents = PluginCacheFile::read_from(
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
        playground.with_files(vec![
            Stub::FileWithContent("config.nu", ""),
            Stub::FileWithContent("env.nu", ""),
        ]);

        let file = File::create(dirs.test().join("test-plugin-file.msgpackz"))
            .expect("failed to create file");
        let mut contents = PluginCacheFile::new();

        contents.upsert_plugin(PluginCacheItem {
            name: "example".into(),
            filename: example_plugin_path,
            shell: None,
            data: PluginCacheItemData::Valid { commands: vec![] },
        });

        contents.upsert_plugin(PluginCacheItem {
            name: "badtest".into(),
            // this doesn't exist, but it should be ok
            filename: dirs.test().join("nu_plugin_badtest"),
            shell: None,
            data: PluginCacheItemData::Invalid,
        });

        contents
            .write_to(file, None)
            .expect("failed to write plugin file");

        let result = Command::new(nu_test_support::fs::executable_path())
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
                "plugin list | get name | to json --raw",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
        assert!(err.contains("cached plugin data"));
        assert!(err.contains("badtest"));
    })
}
