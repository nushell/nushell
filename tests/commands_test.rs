mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn lines() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        r#"open cargo_sample.toml --raw | lines | skip-while $it != "[dependencies]" | skip 1 | first 1 | split-column "=" | get Column1 | trim | echo $it"#
    );

    assert_eq!(output, "rustyline");
}

#[test]
fn save_figures_out_intelligently_where_to_write_out_with_metadata() {
    let sandbox = Playground::setup_for("save_smart_test")
        .with_files(vec![FileWithContent(
            "cargo_sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.1.1"
                authors = ["Yehuda Katz <wycats@gmail.com>"]
                description = "A shell for the GitHub era"
                license = "ISC"
                edition = "2018"
            "#,
        )])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let subject_file = format!("{}/{}", full_path, "cargo_sample.toml");

    nu!(
        _output,
        cwd(&Playground::root()),
        "open save_smart_test/cargo_sample.toml | inc package.version --minor | save"
    );

    let actual = h::file_contents(&subject_file);
    assert!(actual.contains("0.2.0"));
}

#[test]
fn save_can_write_out_csv() {
    let sandbox = Playground::setup_for("save_writes_out_csv_test").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_file = format!("{}/{}", full_path, "cargo_sample.csv");

    nu!(
        _output,
        cwd(&Playground::root()),
        "open ../formats/cargo_sample.toml | inc package.version --minor | get package | save save_writes_out_csv_test/cargo_sample.csv"
    );

    let actual = h::file_contents(&expected_file);
    assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
}
