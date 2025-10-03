use nu_test_support::nu;

#[test]
fn url_join_simple() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "",
                "password": "",
                "host": "localhost",
                "port": "",
            } | url join
        "#);

    assert_eq!(actual.out, "http://localhost");
}

#[test]
fn url_join_with_only_user() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "",
                "host": "localhost",
                "port": "",
            } | url join
        "#);

    assert_eq!(actual.out, "http://usr@localhost");
}

#[test]
fn url_join_with_only_pwd() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "",
                "password": "pwd",
                "host": "localhost",
                "port": "",
            } | url join
        "#);

    assert_eq!(actual.out, "http://localhost");
}

#[test]
fn url_join_with_user_and_pwd() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "port": "",
            } | url join
        "#);

    assert_eq!(actual.out, "http://usr:pwd@localhost");
}

#[test]
fn url_join_with_query() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "query": "par_1=aaa&par_2=bbb"
                "port": "",
            } | url join
        "#);

    assert_eq!(actual.out, "http://usr:pwd@localhost?par_1=aaa&par_2=bbb");
}

#[test]
fn url_join_with_params() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "params": {
                    "par_1": "aaa",
                    "par_2": "bbb"
                },
                "port": "1234",
            } | url join
        "#);

    assert_eq!(
        actual.out,
        "http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb"
    );
}

#[test]
fn url_join_with_same_query_and_params() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "query": "par_1=aaa&par_2=bbb",
                "params": {
                    "par_1": "aaa",
                    "par_2": "bbb"
                },
                "port": "1234",
            } | url join
        "#);

    assert_eq!(
        actual.out,
        "http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb"
    );
}

#[test]
fn url_join_with_different_query_and_params() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "query": "par_1=aaa&par_2=bbb",
                "params": {
                    "par_1": "aaab",
                    "par_2": "bbb"
                },
                "port": "1234",
            } | url join
        "#);

    assert!(
        actual
            .err
            .contains("Mismatch, query string from params is: ?par_1=aaab&par_2=bbb")
    );
    assert!(
        actual
            .err
            .contains("instead query is: ?par_1=aaa&par_2=bbb")
    );

    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "params": {
                    "par_1": "aaab",
                    "par_2": "bbb"
                },
                "query": "par_1=aaa&par_2=bbb",
                "port": "1234",
            } | url join
        "#);

    assert!(
        actual
            .err
            .contains("Mismatch, query param is: par_1=aaa&par_2=bbb")
    );
    assert!(
        actual
            .err
            .contains("instead query string from params is: ?par_1=aaab&par_2=bbb")
    );
}

#[test]
fn url_join_with_invalid_params() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "params": "aaa",
                "port": "1234",
            } | url join
        "#);

    assert!(
        actual
            .err
            .contains("Key params has to be a record or a table")
    );
}

#[test]
fn url_join_with_port() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": "1234",
            } | url join
        "#);

    assert_eq!(actual.out, "http://localhost:1234");

    let actual = nu!(r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": 1234,
            } | url join
        "#);

    assert_eq!(actual.out, "http://localhost:1234");
}

#[test]
fn url_join_with_invalid_port() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": "aaaa",
            } | url join
        "#);

    assert!(
        actual
            .err
            .contains("Port parameter should represent an unsigned int")
    );

    let actual = nu!(r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": [],
            } | url join
        "#);

    assert!(
        actual
            .err
            .contains("Port parameter should be an unsigned int or a string representing it")
    );
}

#[test]
fn url_join_with_missing_scheme() {
    let actual = nu!(r#"
            {
                "host": "localhost"
            } | url join
        "#);

    assert!(actual.err.contains("missing parameter: scheme"));
}

#[test]
fn url_join_with_missing_host() {
    let actual = nu!(r#"
            {
                "scheme": "https"
            } | url join
        "#);

    assert!(actual.err.contains("missing parameter: host"));
}

#[test]
fn url_join_with_fragment() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "fragment": "frag",
                "port": "1234",
            } | url join
        "#);

    assert_eq!(actual.out, "http://usr:pwd@localhost:1234#frag");
}

#[test]
fn url_join_with_fragment_and_params() {
    let actual = nu!(r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "params": {
                    "par_1": "aaa",
                    "par_2": "bbb"
                },
                "port": "1234",
                "fragment": "frag"
            } | url join
        "#);

    assert_eq!(
        actual.out,
        "http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb#frag"
    );
}

#[test]
fn url_join_with_empty_params() {
    let actual = nu!(r#"
        {
            "scheme": "https",
            "host": "localhost",
            "path": "/foo",
            "params": {}
        } | url join
        "#);

    assert_eq!(actual.out, "https://localhost/foo");
}

#[test]
fn url_join_with_list_in_params() {
    let actual = nu!(r#"
        {
            "scheme": "http",
            "username": "usr",
            "password": "pwd",
            "host": "localhost",
            "params": {
                "par_1": "aaa",
                "par_2": ["bbb", "ccc"]
            },
            "port": "1234",
        } | url join
    "#);

    assert_eq!(
        actual.out,
        "http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb&par_2=ccc"
    );
}

#[test]
fn url_join_with_params_table() {
    let actual = nu!(r#"
        {
            "scheme": "http",
            "username": "usr",
            "password": "pwd",
            "host": "localhost",
            "params": [
                ["key", "value"];
                ["par_1", "aaa"],
                ["par_2", "bbb"],
                ["par_1", "ccc"],
                ["par_2", "ddd"],
            ],
            "port": "1234",
        } | url join
    "#);

    assert_eq!(
        actual.out,
        "http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb&par_1=ccc&par_2=ddd"
    );
}

#[test]
fn url_join_with_params_invalid_table() {
    let actual = nu!(r#"
        {
            "scheme": "http",
            "username": "usr",
            "password": "pwd",
            "host": "localhost",
            "params": (
                [
                    { key: foo, value: bar }
                    "not a record"
                ]
            ),
            "port": "1234",
        } | url join
    "#);

    assert!(actual.err.contains("expected a table"));
    assert!(
        actual
            .err
            .contains("not a table, contains non-record values")
    );
}
