use nu_test_support::{nu, pipeline};

#[cfg(feature = "which-support")]
mod environment;

mod pipeline;

//FIXME: jt: we need to focus some fixes on wix as the plugins will differ
#[ignore]
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
            | each { if $it.wix != $it.cargo { 1 } { 0 } }
            | math sum
            "#
    ));

    assert_eq!(actual.out, "0");
}

#[test]
#[cfg(not(windows))]
fn do_not_panic_if_broken_pipe() {
    // `nu -h | false`
    // used to panic with a BrokenPipe error
    let child_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{:?} -h | false",
            nu_test_support::fs::executable_path()
        ))
        .output()
        .expect("failed to execute process");

    assert!(child_output.stderr.is_empty());
}
