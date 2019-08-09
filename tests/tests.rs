mod helpers;

use helpers::in_directory as cwd;
use helpers::normalize_string;

#[test]
fn external_num() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.Height | echo $it"
    );

    assert_eq!(output, "10");
}

#[test]
fn external_has_correct_quotes() {
    nu!(output, cwd("."), r#"echo "hello world""#);

    let output = normalize_string(&output);

    assert_eq!(output, r#""hello world""#);
}

#[test]
fn inc_plugin() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.Height | inc | echo $it"
    );

    assert_eq!(output, "11");
}

#[test]
fn add_plugin() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | add dev-dependencies.newdep \"1\" | get dev-dependencies.newdep | echo $it"
    );

    assert_eq!(output, "1");
}

#[test]
fn edit_plugin() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | edit dev-dependencies.pretty_assertions \"7\" | get dev-dependencies.pretty_assertions | echo $it"
    );

    assert_eq!(output, "7");
}
