mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn lines() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip-while $it != "[dependencies]"
            | skip 1
            | first 1
            | split-column "="
            | get Column1
            | trim
            | echo $it
        "#
    ));

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
            "#,
        )]);

        let subject_file = dirs.test().join("cargo_sample.toml");

        nu!(
            cwd: dirs.root(),
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
            cwd: dirs.root(),
            "open {}/cargo_sample.toml | inc package.version --minor | get package | save save_test_2/cargo_sample.csv",
            dirs.formats()
        );

        let actual = h::file_contents(expected_file);
        assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
    })
}

// This test is more tricky since we are checking for binary output. The output rendered in ASCII is (roughly):
// �authors+0Yehuda Katz <wycats@gmail.com>descriptionA shell for the GitHub eraedition2018licenseISCnamenuversion0.2.0
// It is not valid utf-8, so this is just an approximation.
#[test]
fn save_can_write_out_bson() {
    Playground::setup("save_test_3", |dirs, _| {
        let expected_file = dirs.test().join("cargo_sample.bson");

        nu!(
            cwd: dirs.root(),
            "open {}/cargo_sample.toml | inc package.version --minor | get package | save save_test_3/cargo_sample.bson",
            dirs.formats()
        );

        let actual = h::file_contents_binary(expected_file);
        assert!(
            actual
                == vec![
                    168, 0, 0, 0, 4, 97, 117, 116, 104, 111, 114, 115, 0, 43, 0, 0, 0, 2, 48, 0,
                    31, 0, 0, 0, 89, 101, 104, 117, 100, 97, 32, 75, 97, 116, 122, 32, 60, 119,
                    121, 99, 97, 116, 115, 64, 103, 109, 97, 105, 108, 46, 99, 111, 109, 62, 0, 0,
                    2, 100, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 0, 27, 0, 0, 0, 65,
                    32, 115, 104, 101, 108, 108, 32, 102, 111, 114, 32, 116, 104, 101, 32, 71, 105,
                    116, 72, 117, 98, 32, 101, 114, 97, 0, 2, 101, 100, 105, 116, 105, 111, 110, 0,
                    5, 0, 0, 0, 50, 48, 49, 56, 0, 2, 108, 105, 99, 101, 110, 115, 101, 0, 4, 0, 0,
                    0, 73, 83, 67, 0, 2, 110, 97, 109, 101, 0, 3, 0, 0, 0, 110, 117, 0, 2, 118,
                    101, 114, 115, 105, 111, 110, 0, 6, 0, 0, 0, 48, 46, 50, 46, 48, 0, 0
                ]
        );
    })
}
