use nu_json::Value;
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn txt(text: &str) -> String {
    let out = String::from_utf8_lossy(text.as_bytes());

    #[cfg(windows)]
    {
        out.replace("\r\n", "").replace('\n', "")
    }

    #[cfg(not(windows))]
    {
        out.to_string()
    }
}

fn hjson_expectations() -> PathBuf {
    nu_test_support::fs::assets().join("nu_json").into()
}

fn get_test_content(name: &str) -> io::Result<String> {
    let expectations = hjson_expectations();

    let mut p = format!("{}/{}_test.hjson", expectations.display(), name);

    if !Path::new(&p).exists() {
        p = format!("{}/{}_test.json", expectations.display(), name);
    }

    fs::read_to_string(&p)
}

fn get_result_content(name: &str) -> io::Result<(String, String)> {
    let expectations = hjson_expectations();

    let p1 = format!("{}/{}_result.json", expectations.display(), name);
    let p2 = format!("{}/{}_result.hjson", expectations.display(), name);

    Ok((fs::read_to_string(p1)?, fs::read_to_string(p2)?))
}

// add fixes where rust's json differs from javascript
fn get_fix(s: &str) -> fn(String) -> String {
    fn remove_negative_zero(json: String) -> String {
        json.replace("    -0,", "    0,")
    }

    fn positive_exp_add_sign(json: String) -> String {
        json.replace("1.23456789e34", "1.23456789e+34")
            .replace("2.3456789012e76", "2.3456789012e+76")
    }

    match s {
        "kan" => remove_negative_zero,
        "pass1" => positive_exp_add_sign,
        _ => std::convert::identity,
    }
}

#[rstest]
#[case("charset")]
#[case("comments")]
#[case("empty")]
#[case("failCharset1")]
#[case("failJSON02")]
#[case("failJSON05")]
#[case("failJSON06")]
#[case("failJSON07")]
#[case("failJSON08")]
#[case("failJSON10")]
#[case("failJSON11")]
#[case("failJSON12")]
#[case("failJSON13")]
#[case("failJSON14")]
#[case("failJSON15")]
#[case("failJSON16")]
#[case("failJSON17")]
#[case("failJSON19")]
#[case("failJSON20")]
#[case("failJSON21")]
#[case("failJSON22")]
#[case("failJSON23")]
#[case("failJSON24")]
#[case("failJSON26")]
#[case("failJSON28")]
#[case("failJSON29")]
#[case("failJSON30")]
#[case("failJSON31")]
#[case("failJSON32")]
#[case("failJSON33")]
#[case("failJSON34")]
#[case("failKey1")]
#[case("failKey3")]
#[case("failKey4")]
#[case("failMLStr1")]
#[case("failObj1")]
#[case("failObj2")]
#[case("failObj3")]
#[case("failStr1a")]
#[case("failStr1b")]
#[case("failStr1c")]
#[case("failStr1d")]
#[case("failStr2a")]
#[case("failStr2b")]
#[case("failStr2c")]
#[case("failStr2d")]
#[case("failStr3a")]
#[case("failStr3b")]
#[case("failStr3c")]
#[case("failStr3d")]
#[case("failStr4a")]
#[case("failStr4b")]
#[case("failStr4c")]
#[case("failStr4d")]
#[case("failStr5a")]
#[case("failStr5b")]
#[case("failStr5c")]
#[case("failStr5d")]
#[case("failStr6a")]
#[case("failStr6b")]
#[case("failStr6c")]
#[case("failStr6d")]
#[case("kan")]
#[case("keys")]
#[case("oa")]
#[case("pass1")]
#[case("pass2")]
#[case("pass3")]
#[case("pass4")]
#[case("passSingle")]
#[case("root")]
#[case("stringify1")]
#[case("strings")]
#[case("trail")]
fn test_hjson(#[case] name: &str) {
    let should_fail = name.starts_with("fail");
    let test_content = get_test_content(name).unwrap();
    let data: nu_json::Result<Value> = nu_json::from_str(&test_content);
    assert!(should_fail == data.is_err());

    if !should_fail {
        let udata = data.unwrap();
        let (rjson, _rhjson) = get_result_content(name).unwrap();
        let rjson = txt(&rjson);
        // let rhjson = txt(&rhjson);

        let actual_hjson = nu_json::to_string(&udata).as_deref().map(txt).unwrap();
        let actual_json = serde_json::to_string_pretty(&udata)
            .map(get_fix(name))
            .as_deref()
            .map(txt)
            .unwrap();

        assert_eq!(rjson, actual_hjson);
        assert_eq!(rjson, actual_json);
    }
}
