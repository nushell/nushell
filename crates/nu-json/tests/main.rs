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

fn ident(json: String) -> String {
    // serde_json serializes integers with a superfluous .0 suffix
    // let re = Regex::new(r"(?m)(?P<d>\d)\.0(?P<s>,?)$").unwrap();
    // re.replace_all(&json, "$d$s").to_string()
    json
}

fn fix_kan(json: String) -> String {
    json.replace("    -0,", "    0,")
}

fn fix_pass1(json: String) -> String {
    json.replace("1.23456789e34", "1.23456789e+34")
        .replace("2.3456789012e76", "2.3456789012e+76")
}

#[rstest]
#[case("charset", ident)]
#[case("comments", ident)]
#[case("empty", ident)]
#[case("failCharset1", ident)]
#[case("failJSON02", ident)]
#[case("failJSON05", ident)]
#[case("failJSON06", ident)]
#[case("failJSON07", ident)]
#[case("failJSON08", ident)]
#[case("failJSON10", ident)]
#[case("failJSON11", ident)]
#[case("failJSON12", ident)]
#[case("failJSON13", ident)]
#[case("failJSON14", ident)]
#[case("failJSON15", ident)]
#[case("failJSON16", ident)]
#[case("failJSON17", ident)]
#[case("failJSON19", ident)]
#[case("failJSON20", ident)]
#[case("failJSON21", ident)]
#[case("failJSON22", ident)]
#[case("failJSON23", ident)]
#[case("failJSON24", ident)]
#[case("failJSON26", ident)]
#[case("failJSON28", ident)]
#[case("failJSON29", ident)]
#[case("failJSON30", ident)]
#[case("failJSON31", ident)]
#[case("failJSON32", ident)]
#[case("failJSON33", ident)]
#[case("failJSON34", ident)]
#[case("failKey1", ident)]
#[case("failKey3", ident)]
#[case("failKey4", ident)]
#[case("failMLStr1", ident)]
#[case("failObj1", ident)]
#[case("failObj2", ident)]
#[case("failObj3", ident)]
#[case("failStr1a", ident)]
#[case("failStr1b", ident)]
#[case("failStr1c", ident)]
#[case("failStr1d", ident)]
#[case("failStr2a", ident)]
#[case("failStr2b", ident)]
#[case("failStr2c", ident)]
#[case("failStr2d", ident)]
#[case("failStr3a", ident)]
#[case("failStr3b", ident)]
#[case("failStr3c", ident)]
#[case("failStr3d", ident)]
#[case("failStr4a", ident)]
#[case("failStr4b", ident)]
#[case("failStr4c", ident)]
#[case("failStr4d", ident)]
#[case("failStr5a", ident)]
#[case("failStr5b", ident)]
#[case("failStr5c", ident)]
#[case("failStr5d", ident)]
#[case("failStr6a", ident)]
#[case("failStr6b", ident)]
#[case("failStr6c", ident)]
#[case("failStr6d", ident)]
#[case("kan", fix_kan)]
#[case("keys", ident)]
#[case("oa", ident)]
#[case("pass1", fix_pass1)]
#[case("pass2", ident)]
#[case("pass3", ident)]
#[case("pass4", ident)]
#[case("passSingle", ident)]
#[case("root", ident)]
#[case("stringify1", ident)]
#[case("strings", ident)]
#[case("trail", ident)]
fn test_hjson(#[case] name: &str, #[case] fix: fn(String) -> String) {
    let should_fail = name.starts_with("fail");
    let test_content = get_test_content(name).unwrap();
    let data: nu_json::Result<Value> = nu_json::from_str(&test_content);
    assert!(should_fail == data.is_err());

    if !should_fail {
        let udata = data.unwrap();
        let (rjson, rhjson) = get_result_content(name).unwrap();
        let rjson = txt(&rjson);
        let _rhjson = txt(&rhjson);
        let actual_hjson = nu_json::to_string(&udata).unwrap();
        let actual_hjson = txt(&actual_hjson);
        let actual_json = fix(serde_json::to_string_pretty(&udata).unwrap());
        let actual_json = txt(&actual_json);

        assert_eq!(rjson, actual_hjson);
        assert_eq!(rjson, actual_json);
    }
}
