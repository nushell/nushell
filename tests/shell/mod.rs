use nu_test_support::fs::AbsolutePath;
use nu_test_support::playground::{says, Playground};
use nu_test_support::{nu, pipeline};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[cfg(feature = "directories-support")]
#[cfg(feature = "which-support")]
mod environment;

mod pipeline;

#[should_panic]
#[test]
fn runs_configuration_startup_commands() {
    Playground::setup("init_config_startup_commands_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("startup.toml"));

        nu.with_config(&file);

        assert_that!(nu.pipeline("hello-world"), says().to_stdout("Nu World"));
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
                | get Wix.children.Product.children.0.Directory.children.0
                | where Directory.attributes.Id == "$(var.PlatformProgramFilesFolder)"
                | get Directory.children.Directory.children.0 | last
                | get Directory.children.Component.children
                | each { echo $it | first }
                | skip
                | where File.attributes.Name =~ "nu_plugin"
                | str substring [_, -4] File.attributes.Name
                | get File.attributes.Name
                | sort-by
                | wrap wix
            }
            | default wix _
            | each { if $it.wix != $it.cargo { = 1 } { = 0 } }
            | math sum
            "#
    ));

    assert_eq!(actual.out, "0");
}
