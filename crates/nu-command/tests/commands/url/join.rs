use nu_test_support::prelude::*;

#[test]
fn url_join_simple() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "",
                "password": "",
                "host": "localhost",
                "port": "",
            } | url join
        "#;

    test().run(code).expect_value_eq("http://localhost")
}

#[test]
fn url_join_with_only_user() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "",
                "host": "localhost",
                "port": "",
            } | url join
        "#;

    test().run(code).expect_value_eq("http://usr@localhost")
}

#[test]
fn url_join_with_only_pwd() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "",
                "password": "pwd",
                "host": "localhost",
                "port": "",
            } | url join
        "#;

    test().run(code).expect_value_eq("http://localhost")
}

#[test]
fn url_join_with_user_and_pwd() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "port": "",
            } | url join
        "#;

    test().run(code).expect_value_eq("http://usr:pwd@localhost")
}

#[test]
fn url_join_with_query() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "query": "par_1=aaa&par_2=bbb"
                "port": "",
            } | url join
        "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost?par_1=aaa&par_2=bbb")
}

#[test]
fn url_join_with_params() -> Result {
    let code = r#"
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
        "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb")
}

#[test]
fn url_join_with_same_query_and_params() -> Result {
    let code = r#"
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
        "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb")
}

#[test]
fn url_join_with_different_query_and_params() -> Result {
    let code = r#"
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
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(
                left_message,
                "Mismatch, query string from params is: ?par_1=aaab&par_2=bbb"
            );
            assert_eq!(right_message, "instead query is: ?par_1=aaa&par_2=bbb");
        }
        err => return Err(err.into()),
    }

    let code = r#"
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
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(
                left_message,
                "Mismatch, query param is: par_1=aaa&par_2=bbb"
            );
            assert_eq!(
                right_message,
                "instead query string from params is: ?par_1=aaab&par_2=bbb"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn url_join_with_invalid_params() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "params": "aaa",
                "port": "1234",
            } | url join
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParametersSingle { msg, .. } => {
            assert_eq!(msg, "Key params has to be a record or a table");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn url_join_with_port() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": "1234",
            } | url join
        "#;

    test().run(code).expect_value_eq("http://localhost:1234")?;

    let code = r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": 1234,
            } | url join
        "#;

    test().run(code).expect_value_eq("http://localhost:1234")
}

#[test]
fn url_join_with_invalid_port() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": "aaaa",
            } | url join
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParametersSingle { msg, .. } => {
            assert_eq!(msg, "Port parameter should represent an unsigned int");
        }
        err => return Err(err.into()),
    }

    let code = r#"
            {
                "scheme": "http",
                "host": "localhost",
                "port": [],
            } | url join
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParametersSingle { msg, .. } => {
            assert_eq!(
                msg,
                "Port parameter should be an unsigned int or a string representing it"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn url_join_with_missing_scheme() -> Result {
    let code = r#"
            {
                "host": "localhost"
            } | url join
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::MissingParameter { param_name, .. } => {
            assert_eq!(param_name, "scheme");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn url_join_with_missing_host() -> Result {
    let code = r#"
            {
                "scheme": "https"
            } | url join
        "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::MissingParameter { param_name, .. } => {
            assert_eq!(param_name, "host");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn url_join_with_fragment() -> Result {
    let code = r#"
            {
                "scheme": "http",
                "username": "usr",
                "password": "pwd",
                "host": "localhost",
                "fragment": "frag",
                "port": "1234",
            } | url join
        "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234#frag")
}

#[test]
fn url_join_with_fragment_and_params() -> Result {
    let code = r#"
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
        "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb#frag")
}

#[test]
fn url_join_with_empty_params() -> Result {
    let code = r#"
        {
            "scheme": "https",
            "host": "localhost",
            "path": "/foo",
            "params": {}
        } | url join
        "#;

    test().run(code).expect_value_eq("https://localhost/foo")
}

#[test]
fn url_join_with_list_in_params() -> Result {
    let code = r#"
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
    "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb&par_2=ccc")
}

#[test]
fn url_join_with_params_table() -> Result {
    let code = r#"
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
    "#;

    test()
        .run(code)
        .expect_value_eq("http://usr:pwd@localhost:1234?par_1=aaa&par_2=bbb&par_1=ccc&par_2=ddd")
}

#[test]
fn url_join_with_params_invalid_table() -> Result {
    let code = r#"
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
    "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::UnsupportedInput { msg, input, .. } => {
            assert_eq!(msg, "expected a table");
            assert_eq!(input, "not a table, contains non-record values");
            Ok(())
        }
        err => Err(err.into()),
    }
}
