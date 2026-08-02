use std::{
    fs::File,
    path::{Path, PathBuf},
};

use nu_protocol::{PluginRegistryFile, PluginRegistryItem, PluginRegistryItemData, Record};
use nu_test_support::prelude::*;
use rstest::rstest;

#[derive(Debug)]
struct EmptyConfigs {
    config: PathBuf,
    env: PathBuf,
    plugin: PathBuf,
}

impl EmptyConfigs {
    #[track_caller]
    fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let config = root.join("config.nu");
        let env = root.join("env.nu");
        let plugin = root.join("plugin.msgpackz");
        File::create(&config).unwrap();
        File::create(&env).unwrap();
        File::create(&plugin).unwrap();
        Self {
            config,
            env,
            plugin,
        }
    }

    fn nu(&self) -> String {
        format!(
            "
                let commands = $in
                (
                    nu --no-std-lib
                       --config {config} 
                       --env-config {env} 
                       --plugin-config {plugin} 
                       --commands $commands
                )
            ",
            config = self.config.display(),
            env = self.env.display(),
            plugin = self.plugin.display()
        )
    }
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_then_restart_nu() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();
        let commands = format!("plugin add {}", NU_PLUGIN_EXAMPLE.path().display());
        test().run_with_data(&nu, commands).expect_value_eq("")?;
        let commands = "plugin list --engine | get name | str join ','";
        test()
            .run_with_data(&nu, commands)
            .expect_value_eq("example")
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_in_nu_plugin_dirs_consts() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();

        let commands = format!(
            "
                $env.NU_PLUGIN_DIRS = null
                const NU_PLUGIN_DIRS = ['{}']
                plugin add {}
            ",
            NU_PLUGIN_EXAMPLE.path().parent().unwrap().display(),
            NU_PLUGIN_EXAMPLE.path().file_name().unwrap().display()
        );

        let mut tester = test();
        tester.run_with_data(&nu, commands).expect_value_eq("")?;
        tester
            .run_with_data("open $in | get plugins.name ", configs.plugin.as_path())
            .expect_value_eq(["example"])
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_in_nu_plugin_dirs_env() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();

        let commands = format!(
            "
                $env.NU_PLUGIN_DIRS = ['{}']
                plugin add {}
            ",
            NU_PLUGIN_EXAMPLE.path().parent().unwrap().display(),
            NU_PLUGIN_EXAMPLE.path().file_name().unwrap().display()
        );

        let mut tester = test();
        tester.run_with_data(&nu, commands).expect_value_eq("")?;
        tester
            .run_with_data("open $in | get plugins.name ", configs.plugin.as_path())
            .expect_value_eq(["example"])
    })
}

#[rstest]
#[case::unnested("test-plugin-file.msgpackz")]
#[case::nested("nested/dirs/test-plugin-file.msgpackz")]
#[nu_test_support::test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn plugin_add_to_custom_path(#[case] plugin_config_path_tail: &str) -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let plugin_config = dirs.test().join(plugin_config_path_tail);
        let code = format!(
            "
                plugin add --plugin-config '{plugin_config}' '{plugin}'
                open '{plugin_config}' | get plugins.name
            ",
            plugin_config = plugin_config.display(),
            plugin = NU_PLUGIN_EXAMPLE.path().display(),
        );

        test().run(code).expect_value_eq(["example"])
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE, NU_PLUGIN_INC)]
fn plugin_rm_then_restart_nu() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();
        let mut tester = test();

        // fill plugin config with something
        let () = tester.run(format!(
            "
                plugin add --plugin-config '{plugin_config}' '{example_plugin}'
                plugin add --plugin-config '{plugin_config}' '{inc_plugin}'
                ignore
            ",
            plugin_config = configs.plugin.display(),
            example_plugin = NU_PLUGIN_EXAMPLE.path().display(),
            inc_plugin = NU_PLUGIN_INC.path().display(),
        ))?;

        // remove the plugin
        tester
            .run_with_data(&nu, "plugin rm example")
            .expect_value_eq("")?;

        // verify plugin got removed
        tester
            .run_with_data(&nu, "plugin list | get name | to json --raw")
            .expect_value_eq(r#"["inc"]"#)
    })
}

#[test]
#[deps(NU)]
fn plugin_rm_not_found() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let outcome: Record =
            test().run_with_data(format!("{} | complete", configs.nu()), "plugin rm example")?;
        assert_ne!(outcome["exit_code"], Value::test_int(0));
        assert_contains("example", outcome["stderr"].as_str().unwrap());
        Ok(())
    })
}

#[rstest]
#[case::by_name("example".to_string())]
#[case::by_filename(NU_PLUGIN_EXAMPLE.path().display().to_string())]
#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE, NU_PLUGIN_INC)]
fn plugin_rm_from_custom_path(#[case] plugin_to_remove: String) -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let mut tester = test();

        // fill plugin config with something
        let () = tester.run(format!(
            "
                plugin add --plugin-config '{plugin_config}' '{example_plugin}'
                plugin add --plugin-config '{plugin_config}' '{inc_plugin}'
                ignore
            ",
            plugin_config = configs.plugin.display(),
            example_plugin = NU_PLUGIN_EXAMPLE.path().display(),
            inc_plugin = NU_PLUGIN_INC.path().display(),
        ))?;

        let () = tester.run(format!(
            "plugin rm --plugin-config '{}' '{}'",
            configs.plugin.display(),
            plugin_to_remove,
        ))?;

        tester
            .run_with_data("open $in | get plugins.name", configs.plugin.as_path())
            .expect_value_eq(["inc"])
    })
}

/// Running nu with a test plugin file that fails to parse on one plugin should just cause a warning
/// but the others should be loaded
#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn warning_on_invalid_plugin_item() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());

        let mut registry = PluginRegistryFile::new();
        registry.upsert_plugin(PluginRegistryItem {
            name: "example".into(),
            filename: NU_PLUGIN_EXAMPLE.path(),
            shell: None,
            data: PluginRegistryItemData::Valid {
                metadata: Default::default(),
                commands: Default::default(),
            },
        });
        registry.upsert_plugin(PluginRegistryItem {
            name: "badtest".into(),
            filename: dirs.test().join("nu_plugin_badtest").into(),
            shell: None,
            data: PluginRegistryItemData::Invalid,
        });
        registry
            .write_to(File::create(configs.plugin.as_path()).unwrap(), None)
            .unwrap();

        let outcome: Record = test().run_with_data(
            format!("{} | complete", configs.nu()),
            "plugin list --engine | get name | to nuon --raw",
        )?;

        assert_eq!(outcome["exit_code"], Value::test_int(0));
        assert_eq!(outcome["stdout"], Value::test_string("[example]\n"));
        let stderr = outcome["stderr"].as_str().unwrap();
        assert_contains("registered plugin data", stderr);
        assert_contains("badtest", stderr);
        assert_contains("Failed to load 1 plugin entry", stderr);

        Ok(())
    })
}

#[test]
#[deps(NU)]
fn plugin_use_error_not_found() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        PluginRegistryFile::new()
            .write_to(File::create(&configs.plugin).unwrap(), None)
            .unwrap();

        let outcome: Record = test().run_with_data(
            format!("{} | complete", configs.nu()),
            "plugin use custom_values",
        )?;
        assert_contains("Plugin not found", outcome["stderr"].as_str().unwrap());

        Ok(())
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_shows_up_in_default_plugin_list_after_add() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let commands = format!(
            "plugin add '{}'; plugin list | get status | to nuon --raw",
            NU_PLUGIN_EXAMPLE.path().display()
        );
        test()
            .run_with_data(configs.nu(), commands)
            .expect_value_eq("[added]")
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_shows_removed_after_removing() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let mut tester = test();

        let () = tester.run(format!(
            "plugin add --plugin-config '{}' '{}' | null",
            configs.plugin.display(),
            NU_PLUGIN_EXAMPLE.path().display()
        ))?;

        test()
            .run_with_data(
                configs.nu(),
                "plugin rm example; plugin list | get status | to nuon --raw",
            )
            .expect_value_eq("[removed]")
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_and_then_use() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();
        let mut tester = test();

        let plugin_add = format!("plugin add '{}'", NU_PLUGIN_EXAMPLE.path().display());
        test()
            .run_with_data(configs.nu(), plugin_add)
            .expect_value_eq("")?;

        let plugin_use = "
            plugin use example
            plugin list --engine | get name | to nuon --raw
        ";
        tester
            .run_with_data(&nu, plugin_use)
            .expect_value_eq("[example]")
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_and_then_use_by_filename() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();
        let mut tester = test();

        let plugin_add = format!("plugin add '{}'", NU_PLUGIN_EXAMPLE.path().display());
        test()
            .run_with_data(configs.nu(), plugin_add)
            .expect_value_eq("")?;

        let plugin_use = format!(
            "
                plugin use '{}'
                plugin list --engine | get name | to nuon --raw
            ",
            NU_PLUGIN_EXAMPLE.path().display()
        );
        tester
            .run_with_data(&nu, plugin_use)
            .expect_value_eq("[example]")
    })
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn plugin_add_then_use_with_custom_path() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let configs = EmptyConfigs::new(dirs.test());
        let nu = configs.nu();
        let mut tester = test();

        let plugin_add = format!(
            "plugin add --plugin-config '{}' '{}'",
            configs.plugin.display(),
            NU_PLUGIN_EXAMPLE.path().display()
        );
        test()
            .run_with_data(configs.nu(), plugin_add)
            .expect_value_eq("")?;

        let plugin_use = format!(
            "
                plugin use --plugin-config '{0}' example
                plugin list --engine | get name | to nuon --raw
            ",
            configs.plugin.display()
        );
        tester
            .run_with_data(&nu, plugin_use)
            .expect_value_eq("[example]")
    })
}
