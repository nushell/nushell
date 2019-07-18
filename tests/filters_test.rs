mod helpers;

use helpers::in_directory as cwd;

#[test]
fn can_convert_table_to_json_text_and_from_json_text_back_into_table() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | to-json | from-json | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee | echo $it");

    assert_eq!(output, "markup");
}

#[test]
fn can_convert_table_to_toml_text_and_from_toml_text_back_into_table() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | to-toml | from-toml | get package.name | echo $it"
    );

    assert_eq!(output, "nu");
}

#[test]
fn can_convert_table_to_yaml_text_and_from_yaml_text_back_into_table() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open appveyor.yml | to-yaml | from-yaml | get environment.global.PROJECT_NAME | echo $it"
    );

    assert_eq!(output, "nushell");
}

#[test]
fn can_sort_by_column() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip 1 | first 4 | split-column \"=\" | sort-by Column1 | skip 1 | first 1 | get Column1 | trim | echo $it");

    assert_eq!(output, "description");
}

#[test]
fn can_split_by_column() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip 1 | first 1 | split-column \"=\" | get Column1 | trim | echo $it");

    assert_eq!(output, "name");
}

#[test]
fn can_inc_version() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.version --minor | get package.version | echo $it"
    );

    assert_eq!(output, "0.2.0");
}

#[test]
fn can_inc_field() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.edition | get package.edition | echo $it"
    );

    assert_eq!(output, "2019");
}

#[test]
fn can_filter_by_unit_size_comparison() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "ls | where size > 1kb | get name | trim | echo $it"
    );

    assert_eq!(output, "cargo_sample.toml");
}
