mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn nth_selects_a_row() {
    Playground::setup("nth_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | sort-by name
                | nth 0
                | get name
                | echo $it
            "#
        ));

        assert_eq!(actual, "arepas.txt");
    });
}

#[test]
fn nth_selects_many_rows() {
    Playground::setup("nth_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | get name
                | nth 1 0
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    });
}
#[test]
fn default_row_data_if_column_missing() {
    Playground::setup("default_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.json",
            r#"
                {
                    "amigos": [
                        {"name":   "Yehuda"},
                        {"name": "Jonathan", "rusty_luck": 0},
                        {"name":   "Andres", "rusty_luck": 0},
                        {"name":"GorbyPuff"}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_amigos.json
                | get amigos
                | default rusty_luck 1
                | get rusty_luck
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    });
}
#[test]
fn compact_rows_where_given_column_is_empty() {
    Playground::setup("compact_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.json",
            r#"
                {
                    "amigos": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "Jonathan", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff"}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_amigos.json
                | get amigos
                | compact rusty_luck
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    });
}
#[test]
fn compact_empty_rows_by_default() {
    Playground::setup("compact_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                echo "[1,2,3,14,null]"
                | from-json
                | compact
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "4");
    });
}
#[test]
fn group_by() {
    Playground::setup("group_by_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.csv
                | group-by rusty_at
                | get "10/11/2013"
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    })
}

#[test]
fn histogram() {
    Playground::setup("histogram_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                Jonathan,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.csv
                | histogram rusty_at countries
                | where rusty_at == "Ecuador"
                | get countries
                | echo $it
            "#
        ));

        assert_eq!(actual, "**************************************************");
        // 50%
    })
}

#[test]
fn group_by_errors_if_unknown_column_name() {
    Playground::setup("group_by_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.csv
                | group-by ttype
            "#
        ));

        assert!(actual.contains("Unknown column"));
    })
}

#[test]
fn split_by() {
    Playground::setup("split_by_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.csv
                | group-by rusty_at
                | split-by type
                | get A."10/11/2013"
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    })
}

#[test]
fn split_by_errors_if_no_table_given_as_input() {
    Playground::setup("split_by_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu_error!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | get name
                | split-by type
            "#
        ));

        assert!(actual.contains("Expected table from pipeline"));
    })
}

#[test]
fn first_gets_first_rows_by_amount() {
    Playground::setup("first_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | first 3
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn first_gets_all_rows_if_amount_higher_than_all_rows() {
    Playground::setup("first_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | first 99
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "4");
    })
}

#[test]
fn first_gets_first_row_when_no_amount_given() {
    Playground::setup("first_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | first
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "1");
    })
}

#[test]
fn last_gets_last_rows_by_amount() {
    Playground::setup("last_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | last 3
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn last_gets_last_row_when_no_amount_given() {
    Playground::setup("last_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                ls
                | last
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "1");
    })
}

#[test]
fn get() {
    Playground::setup("get_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                nu_party_venue = "zion"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get nu_party_venue
                | echo $it
            "#
        ));

        assert_eq!(actual, "zion");
    })
}

#[test]
fn get_more_than_one_member() {
    Playground::setup("get_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [[fortune_tellers]]
                name = "Andrés N. Robalino"
                arepas = 1
                broken_builds = 0

                [[fortune_tellers]]
                name = "Jonathan Turner"
                arepas = 1
                broken_builds = 1

                [[fortune_tellers]]
                name = "Yehuda Katz"
                arepas = 1
                broken_builds = 1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get fortune_tellers
                | get arepas broken_builds
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "5");
    })
}

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
fn it_arg_works_with_many_inputs_to_external_command() {
    Playground::setup("it_arg_works_with_many_inputs", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContent("file1", "text"),
            FileWithContent("file2", " and more text"),
        ]);

        let (stdout, stderr) = nu_combined!(
            cwd: dirs.test(), h::pipeline(
            r#"
                echo hello world
                | split-row " "
                | ^echo $it
            "#
        ));

        #[cfg(windows)]
        assert_eq!("hello world", stdout);

        #[cfg(not(windows))]
        assert_eq!("helloworld", stdout);

        assert!(!stderr.contains("No such file or directory"));
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
        assert!(actual.contains("[Table],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
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
