use nu_json::Value;
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn txt(text: String) -> String {
    let out = text;

    #[cfg(windows)]
    {
        out.replace("\r\n", "\n")
    }

    #[cfg(not(windows))]
    {
        out
    }
}

// This test will fail if/when `nu_test_support::fs::assets()`'s return value changes.
#[rstest]
fn assert_rstest_finds_assets(#[files("../../tests/assets/nu_json")] rstest_supplied: PathBuf) {
    // rstest::files runs paths through `fs::canonicalize`, which:
    // > On Windows, this converts the path to use extended length path syntax
    // So we make sure to canonicalize both paths.
    assert_eq!(
        fs::canonicalize(rstest_supplied).unwrap(),
        fs::canonicalize(nu_test_support::fs::assets().join("nu_json")).unwrap()
    );
}

#[rstest]
fn test_hjson_fails(#[files("../../tests/assets/nu_json/fail*_test.*")] file: PathBuf) {
    let contents = fs::read_to_string(file).unwrap();
    let data: nu_json::Result<Value> = nu_json::from_str(&contents);
    assert!(data.is_err());
}

#[rstest]
fn test_hjson(
    #[files("../../tests/assets/nu_json/*_test.*")]
    #[exclude("fail*")]
    test_file: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = test_file
        .file_stem()
        .and_then(|x| x.to_str())
        .and_then(|x| x.strip_suffix("_test"))
        .unwrap();

    let data: Value = nu_json::from_str(fs::read_to_string(&test_file)?.as_str())?;

    let r_json = get_content(get_result_path(&test_file, "json").as_deref().unwrap())?;
    // let r_hjson = get_content(get_result_path(&test_file, "hjson").as_deref().unwrap())?;
    let r_hjson = r_json.as_str();

    let actual_json = serde_json::to_string_pretty(&data).map(get_fix(name))?;
    let actual_hjson = nu_json::to_string(&data).map(txt)?;

    assert_eq!(r_json, actual_json);
    assert_eq!(r_hjson, actual_hjson);

    Ok(())
}

fn get_result_path(test_file: &Path, ext: &str) -> Option<PathBuf> {
    let name = test_file
        .file_stem()
        .and_then(|x| x.to_str())
        .and_then(|x| x.strip_suffix("_test"))?;

    Some(test_file.with_file_name(format!("{name}_result.{ext}")))
}

fn get_content(file: &Path) -> io::Result<String> {
    fs::read_to_string(file).map(txt)
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
