use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn removes_duplicate_rows() {
    Playground::setup("uniq_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | uniq
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn uniq_values() {
    Playground::setup("uniq_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | pick type
                | uniq
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    })
}

#[test]
fn nested_json_structures() {
    Playground::setup("uniq_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nested_json_structures.json",
            r#"
            [ 
                {
                  "name": "this is duplicated",
                  "nesting": [ { "a": "a", "b": "b" },
                               { "c": "c", "d": "d" }
                  ],
                  "can_be_ordered_differently": {
                    "array": [1, 2, 3, 4, 5],
                    "something": { "else": "works" }
                  }
                },
                {
                  "can_be_ordered_differently": {
                    "something": { "else": "works" },
                    "array": [1, 2, 3, 4, 5]
                  },
                  "nesting": [ { "b": "b", "a": "a" },
                               { "d": "d", "c": "c" }
                  ],
                  "name": "this is duplicated"
                },
                {
                  "name": "this is unique",
                  "nesting": [ { "a": "b", "b": "a" },
                               { "c": "d", "d": "c" }
                  ],
                  "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "does not work" }
                  }
                },
                {
                  "name": "this is unique",
                  "nesting": [ { "a": "a", "b": "b", "c": "c" },
                               { "d": "d", "e": "e", "f": "f" }
                  ],
                  "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "works" }
                  }
                }
              ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nested_json_structures.json
                | uniq
                | count
                | echo $it
            "#
        ));
        assert_eq!(actual, "3");
    })
}

#[test]
fn uniq_when_keys_out_of_order() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": "a", "b": [1,2,3]},{"b": [1,2,3], "a": "a"}]'
            | from-json
            | uniq
            | count
            | echo $it
        "#
    ));

    assert_eq!(actual, "1");
}
