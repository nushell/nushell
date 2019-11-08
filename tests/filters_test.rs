mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn can_convert_table_to_csv_text_and_from_csv_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.csv | to-csv | from-csv | first 1 | get origin | echo $it"
    );

    assert_eq!(actual, "SPAIN");
}

#[test]
fn converts_structured_table_to_csv_text() {
    Playground::setup("filter_to_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open csv_text_sample.txt
                | lines
                | split-column "," a b c d origin
                | last 1
                | to-csv
                | lines
                | nth 1
                | echo $it
            "#
        ));

        assert!(actual.contains("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia"));
    })
}

#[test]
fn converts_structured_table_to_csv_text_skipping_headers_after_conversion() {
    Playground::setup("filter_to_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open csv_text_sample.txt
                | lines
                | split-column "," a b c d origin
                | last 1
                | to-csv --headerless
                | echo $it
            "#
        ));

        assert!(actual.contains("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia"));
    })
}

#[test]
fn converts_from_csv_text_to_structured_table() {
    Playground::setup("filter_from_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.txt
                | from-csv
                | get rusty_luck
                | str --to-int
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn converts_from_csv_text_with_separator_to_structured_table() {
    Playground::setup("filter_from_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name;last_name;rusty_luck
                Andrés;Robalino;1
                Jonathan;Turner;1
                Yehuda;Katz;1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.txt
                | from-csv --separator ';'
                | get rusty_luck
                | str --to-int
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn converts_from_csv_text_skipping_headers_to_structured_table() {
    Playground::setup("filter_from_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_amigos.txt
                | from-csv --headerless
                | get Column3
                | str --to-int
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn can_convert_table_to_json_text_and_from_json_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sgml_description.json
            | to-json
            | from-json
            | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
            | echo $it
        "#
    ));

    assert_eq!(actual, "markup");
}

#[test]
fn converts_from_json_text_to_structured_table() {
    Playground::setup("filter_from_json_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {
                    "katz": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "Jonathan", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff", "rusty_luck": 1}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.txt | from-json | get katz | get rusty_luck | sum | echo $it"
        );

        assert_eq!(actual, "4");
    })
}

#[test]
fn converts_from_json_text_recognizing_objects_independendtly_to_structured_table() {
    Playground::setup("filter_from_json_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {"name":   "Yehuda", "rusty_luck": 1}
                {"name": "Jonathan", "rusty_luck": 1}
                {"name":   "Andres", "rusty_luck": 1}
                {"name":"GorbyPuff", "rusty_luck": 3}
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open katz.txt
                | from-json --objects
                | where name == "GorbyPuff"
                | get rusty_luck
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn converts_structured_table_to_json_text() {
    Playground::setup("filter_to_json_test", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.txt",
            r#"
                JonAndrehudaTZ,3
                GorbyPuff,100
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.txt
                | lines
                | split-column "," name luck
                | pick name
                | to-json
                | from-json
                | nth 0
                | get name
                | echo $it
            "#
        ));

        assert_eq!(actual, "JonAndrehudaTZ");
    })
}

#[test]
fn can_convert_table_to_tsv_text_and_from_tsv_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.tsv | to-tsv | from-tsv | first 1 | get origin | echo $it"
    );

    assert_eq!(actual, "SPAIN");
}

#[test]
fn converts_structured_table_to_tsv_text() {
    Playground::setup("filter_to_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer	shipper	tariff_item	name	origin
                Plasticos Rival	Reverte	2509000000	Calcium carbonate	Spain
                Tigre Ecuador	OMYA Andina	3824909999	Calcium carbonate	Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open tsv_text_sample.txt
                | lines
                | split-column "\t" a b c d origin
                | last 1
                | to-tsv
                | lines
                | nth 1
                | echo $it
            "#
        ));

        assert!(actual.contains("Colombia"));
    })
}

#[test]
fn converts_structured_table_to_tsv_text_skipping_headers_after_conversion() {
    Playground::setup("filter_to_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer    shipper tariff_item name    origin
                Plasticos Rival Reverte 2509000000  Calcium carbonate   Spain
                Tigre Ecuador   OMYA Andina 3824909999  Calcium carbonate   Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open tsv_text_sample.txt
                | lines
                | split-column "\t" a b c d origin
                | last 1
                | to-tsv --headerless
                | echo $it
            "#
        ));

        assert!(actual.contains("Colombia"));
    })
}

#[test]
fn converts_from_tsv_text_to_structured_table() {
    Playground::setup("filter_from_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                first Name	Last Name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_amigos.txt
                | from-tsv
                | get rusty_luck
                | str --to-int
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn converts_from_tsv_text_skipping_headers_to_structured_table() {
    Playground::setup("filter_from_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                first Name	Last Name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_amigos.txt
                | from-tsv --headerless
                | get Column3
                | str --to-int
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn converts_from_ssv_text_to_structured_table() {
    Playground::setup("filter_from_ssv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                NAME              LABELS                                    SELECTOR                  IP              PORT(S)
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open oc_get_svc.txt
                | from-ssv
                | nth 0
                | get IP
                | echo $it
            "#
        ));

        assert_eq!(actual, "172.30.78.158");
    })
}

#[test]
fn converts_from_ssv_text_to_structured_table_with_separator_specified() {
    Playground::setup("filter_from_ssv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                NAME              LABELS                                    SELECTOR                  IP              PORT(S)
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open oc_get_svc.txt
                | from-ssv --minimum-spaces 3
                | nth 0
                | get IP
                | echo $it
            "#
        ));

        assert_eq!(actual, "172.30.78.158");
    })
}

#[test]
fn converts_from_ssv_text_skipping_headers_to_structured_table() {
    Playground::setup("filter_from_ssv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "oc_get_svc.txt",
            r#"
                NAME              LABELS                                    SELECTOR                  IP              PORT(S)
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open oc_get_svc.txt
                | from-ssv --headerless
                | nth 2
                | get Column2
                | echo $it
            "#
        ));

        assert_eq!(actual, "component=apiserver,provider=kubernetes");
    })
}

#[test]
fn can_convert_table_to_bson_and_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sample.bson
            | to-bson
            | from-bson
            | get root
            | nth 1
            | get b
            | echo $it
        "#
    ));

    assert_eq!(actual, "whel");
}

#[test]
fn can_convert_table_to_sqlite_and_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sample.db
            | to-sqlite
            | from-sqlite
            | get table_values
            | nth 2
            | get x
            | echo $it
        "#
    ));

    assert_eq!(actual, "hello");
}

#[test]
fn can_convert_table_to_toml_text_and_from_toml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open cargo_sample.toml
            | to-toml
            | from-toml
            | get package.name
            | echo $it
        "#
    ));

    assert_eq!(actual, "nu");
}

#[test]
fn can_convert_table_to_yaml_text_and_from_yaml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open appveyor.yml
            | to-yaml
            | from-yaml
            | get environment.global.PROJECT_NAME
            | echo $it
        "#
    ));

    assert_eq!(actual, "nushell");
}

#[test]
fn can_encode_and_decode_urlencoding() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
            r#"
                open sample.url
                | to-url
                | from-url
                | get cheese
                | echo $it
            "#
    ));

    assert_eq!(actual, "comté");
}

#[test]
fn can_sort_by_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 4
            | split-column "="
            | sort-by Column1
            | skip 1
            | first 1
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert_eq!(actual, "description");
}

#[test]
fn can_split_by_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 1
            | split-column "="
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert_eq!(actual, "name");
}

#[test]
fn can_sum() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sgml_description.json
            | get glossary.GlossDiv.GlossList.GlossEntry.Sections
            | sum
            | echo $it
        "#
    ));

    assert_eq!(actual, "203")
}

#[test]
fn can_average_numbers() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sgml_description.json
            | get glossary.GlossDiv.GlossList.GlossEntry.Sections
            | average
            | echo $it
        "#
    ));

    assert_eq!(actual, "101.5000000000000")
}

#[test]
fn can_average_bytes() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | skip 1 | first 2 | get size | average | echo $it"
    );

    assert_eq!(actual, "1600.000000000000");
}

#[test]
fn can_filter_by_unit_size_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | where size > 1kb | sort-by size | get name | first 1 | trim | echo $it"
    );

    assert_eq!(actual, "cargo_sample.toml");
}

#[test]
fn can_get_last() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | last 1 | get name | trim | echo $it"
    );

    assert_eq!(actual, "utf16.ini");
}

#[test]
fn can_get_reverse_first() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | reverse | first 1 | get name | trim | echo $it"
    );

    assert_eq!(actual, "utf16.ini");
}

#[test]
fn embed() {
    Playground::setup("embed_test", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name
                Andrés,Robalino
                Jonathan,Turner
                Yehuda,Katz
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.txt
                | from-csv
                | embed caballeros
                | get caballeros
                | nth 0
                | get last_name
                | echo $it
            "#
        ));

        assert_eq!(actual, "Robalino");
    })
}

#[test]
fn get() {
    Playground::setup("get_test", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name
                Andrés,Robalino
                Jonathan,Turner
                Yehuda,Katz
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open los_tres_caballeros.txt
                | from-csv
                | nth 1
                | get last_name
                | echo $it
            "#
        ));

        assert_eq!(actual, "Turner");
    })
}
