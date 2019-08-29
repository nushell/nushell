mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn lines() {
    let actual = nu!(
        cwd("tests/fixtures/formats"),
        r#"open cargo_sample.toml --raw | lines | skip-while $it != "[dependencies]" | skip 1 | first 1 | split-column "=" | get Column1 | trim | echo $it"#
    );

    assert_eq!(actual, "rustyline");
}

#[test]
fn save_figures_out_intelligently_where_to_write_out_with_metadata() {
    Playground::setup("save_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "cargo_sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.1.1"
                authors = ["Yehuda Katz <wycats@gmail.com>"]
                description = "A shell for the GitHub era"
                license = "ISC"
                edition = "2018"
            "#)
        ]);

        let subject_file = dirs.test().join("cargo_sample.toml");

        nu!(
            cwd(dirs.root()),
            "open save_test_1/cargo_sample.toml | inc package.version --minor | save"
        );

        let actual = h::file_contents(&subject_file);
        assert!(actual.contains("0.2.0"));
    })
}

#[test]
fn save_can_write_out_csv() {
    Playground::setup("save_test_2", |dirs, _| {
        let expected_file = dirs.test().join("cargo_sample.csv");

        nu!(
            dirs.root(),
            "open {}/cargo_sample.toml | inc package.version --minor | get package | save save_test_2/cargo_sample.csv",
            dirs.formats()
        );

        let actual = h::file_contents(expected_file);
        assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
    })
}
