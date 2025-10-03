use nu_test_support::nu;

#[test]
fn url_parse_simple() {
    let actual = nu!(r#"
        (
            "https://www.abc.com" | url parse
        ) == {
            scheme: 'https',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '',
            path: '/',
            query: '',
            fragment: '',
            params: []
        }
    "#);
    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_port() {
    let actual = nu!(r#"
        (
            "https://www.abc.com:8011" | url parse
        ) == {
            scheme: 'https',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '8011',
            path: '/',
            query: '',
            fragment: '',
            params: []
        }
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_path() {
    let actual = nu!(r#"
        (
            "http://www.abc.com:8811/def/ghj" | url parse
        ) == {
            scheme: 'http',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '8811',
            path: '/def/ghj',
            query: '',
            fragment: '',
            params: []
        }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_params() {
    let actual = nu!(r#"
        (
            "http://www.abc.com:8811/def/ghj?param1=11&param2=" | url parse
        ) == {
            scheme: 'http',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '8811',
            path: '/def/ghj',
            query: 'param1=11&param2=',
            fragment: '',
            params: [[key, value]; ["param1", "11"], ["param2", ""]]
        }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_duplicate_params() {
    let actual = nu!(r#"
        (
            "http://www.abc.com:8811/def/ghj?param1=11&param2=&param1=22" | url parse
        ) == {
            scheme: 'http',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '8811',
            path: '/def/ghj',
            query: 'param1=11&param2=&param1=22',
            fragment: '',
            params: [[key, value]; ["param1", "11"], ["param2", ""], ["param1", "22"]]
        }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_fragment() {
    let actual = nu!(r#"
        (
            "http://www.abc.com:8811/def/ghj?param1=11&param2=#hello-fragment" | url parse
        ) == {
            scheme: 'http',
            username: '',
            password: '',
            host: 'www.abc.com',
            port: '8811',
            path: '/def/ghj',
            query: 'param1=11&param2=',
            fragment: 'hello-fragment',
            params: [[key, value]; ["param1", "11"], ["param2", ""]]
        }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_with_username_and_password() {
    let actual = nu!(r#"
        (
            "http://user123:password567@www.abc.com:8811/def/ghj?param1=11&param2=#hello-fragment" | url parse
        ) == {
            scheme: 'http',
            username: 'user123',
            password: 'password567',
            host: 'www.abc.com',
            port: '8811',
            path: '/def/ghj',
            query: 'param1=11&param2=',
            fragment: 'hello-fragment',
            params: [[key, value]; ["param1", "11"], ["param2", ""]]
        }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn url_parse_error_empty_url() {
    let actual = nu!(r#""" | url parse"#);

    assert!(actual.err.contains(
        "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
    ));
}
