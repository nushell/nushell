use nu_test_support::{nu, pipeline};

#[test]
fn groups() {
    let sample = r#"
                [[first_name, last_name, rusty_at, type];
                 [Andrés, Robalino, "10/11/2013", A],
                 [JT, Turner, "10/12/2013", B],
                 [Yehuda, Katz, "10/11/2013", A]]
            "#;

    let actual = nu!(pipeline(&format!(
        r#"
                {sample}
                | group-by rusty_at
                | get "10/11/2013"
                | length
            "#
    )));

    assert_eq!(actual.out, "2");
}

#[test]
fn errors_if_given_unknown_column_name() {
    let sample = r#"
                {
                    "nu": {
                        "committers": [
                            {"name": "Andrés N. Robalino"},
                            {"name": "JT Turner"},
                            {"name": "Yehuda Katz"}
                        ],
                        "releases": [
                            {"version": "0.2"}
                            {"version": "0.8"},
                            {"version": "0.9999999"}
                        ],
                        "0xATYKARNU": [
                            ["Th", "e", " "],
                            ["BIG", " ", "UnO"],
                            ["punto", "cero"]
                        ]
                    }
                }
            "#;

    let actual = nu!(pipeline(&format!(
        r#"
                '{sample}'
                | from json
                | group-by {{|| get nu.releases.version }}
            "#
    )));

    assert!(actual.err.contains("can't convert list<string> to string"));
}

#[test]
fn errors_if_column_not_found() {
    let sample = r#"
                [[first_name, last_name, rusty_at, type];
                 [Andrés, Robalino, "10/11/2013", A],
                 [JT, Turner, "10/12/2013", B],
                 [Yehuda, Katz, "10/11/2013", A]]
            "#;

    let actual = nu!(pipeline(&format!("{sample} | group-by ttype")));

    assert!(actual.err.contains("did you mean 'type'"),);
}

#[test]
fn group_by_on_empty_list_returns_empty_record() {
    let actual = nu!("[[a b]; [1 2]] | where false | group-by a");
    assert!(actual.err.is_empty());
    assert!(actual.out.contains("empty record"));
}

#[test]
fn optional_cell_path_works() {
    let actual = nu!("[{foo: 123}, {foo: 234}, {bar: 345}] | group-by foo? | to nuon");
    let expected = r#"{"123": [[foo]; [123]], "234": [[foo]; [234]]}"#;
    assert_eq!(actual.out, expected)
}
