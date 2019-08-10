mod helpers;

use helpers::in_directory as cwd;

#[test]
fn regular_field_by_one() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.edition | get package.edition | echo $it"
    );

    assert_eq!(output, "2019");
}


#[test]
fn by_one_without_passing_field() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | get package.edition | inc | echo $it"
    );

    assert_eq!(output, "2019");
}

#[test]
fn can_only_apply_one() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | first 1 | inc package.version --major --minor"
    );

    assert!(output.contains("Usage: inc field [--major|--minor|--patch]"));
}

#[test]
fn semversion_major() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.version --major | get package.version | echo $it"
    );

    assert_eq!(output, "1.0.0");
}

#[test]
fn semversion_minor() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.version --minor | get package.version | echo $it"
    );

    assert_eq!(output, "0.2.0");
}

#[test]
fn semversion_patch() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | inc package.version --patch | get package.version | echo $it"
    );

    assert_eq!(output, "0.1.2");
}