use nu_test_support::fs::AbsolutePath;
use nu_test_support::playground::{says, Playground};
use nu_test_support::{nu, pipeline};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[cfg(feature = "directories-support")]
#[cfg(feature = "which-support")]
mod environment;

mod pipeline;

#[test]
fn runs_configuration_startup_commands() {
    Playground::setup("init_config_startup_commands_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.config_fixtures().join("startup.toml"));

        nu.with_config(&file);

        assert_that!(nu.pipeline("hello-world"), says().stdout("Nu World"));
    });
}

#[test]
fn plugins_are_declared_with_wix() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            open Cargo.toml
            | get bin.name
            | str find-replace "nu_plugin_(extra|core)_(.*)" "nu_plugin_$2"
            | drop
            | sort-by
            | wrap cargo | merge {
                open wix/main.wxs --raw | from xml
                | get Wix.Product.0.Directory.0
                | where Directory.Id == "$(var.PlatformProgramFilesFolder)"
                | get Directory.Directory.0 | last
                | get Directory.Component
                | each { echo $it | first }
                | skip
                | where File.Name =~ "nu_plugin"
                | str substring [_, -4] File.Name
                | get File.Name
                | sort-by
                | wrap wix
            }
            | default wix _
            | each { if $it.wix != $it.cargo { 1 } { 0 } }
            | math sum
            "#
    ));

    assert_eq!(actual.out, "0");
}
