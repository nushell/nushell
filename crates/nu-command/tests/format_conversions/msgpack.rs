use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::LazyLock};

use chrono::DateTime;
use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;

static GENERATE: LazyLock<PathBuf> = LazyLock::new(|| {
    nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join("generate.nu")
        .into()
});

fn msgpack_test<T: FromValue>(fixture_name: impl AsRef<str>) -> Result<T> {
    msgpack_test_with_opts(fixture_name, "")
}

fn msgpack_test_with_opts<T: FromValue>(
    fixture_name: impl AsRef<str>,
    opts: impl AsRef<str>,
) -> Result<T> {
    let fixture_name = fixture_name.as_ref();
    let opts = opts.as_ref();

    let topic = format!("msgpack test {fixture_name}");
    Playground::setup(&topic, |dirs, _| {
        let mut tester = test().cwd(dirs.test());

        let generate = format!("use {}; generate main {fixture_name}", GENERATE.display());
        let _: Value = tester
            .run(&generate)
            .expect("could not generate msgpack fixture");

        let open = format!("open {fixture_name}.msgpack --raw | from msgpack {opts}");
        tester.run(&open)
    })
}

#[test]
fn sample() -> Result {
    let values: Vec<Value> = msgpack_test("sample")?;
    let mut values = values.into_iter();
    let values = &mut values;

    fn assert_next<T: FromValue + Debug + PartialEq>(
        values: &mut impl Iterator<Item = Value>,
        comparison: T,
    ) -> Result {
        let expect_msg = format!("expected next value to compare against {comparison:?}");
        let value = T::from_value(values.next().expect(&expect_msg))?;
        assert_eq!(value, comparison);
        Ok(())
    }

    assert_next(values, ())?;
    assert_next(values, false)?;
    assert_next(values, true)?;
    assert_next(values, 17i8)?;
    assert_next(values, -2i8)?;
    assert_next(values, 34u16)?; // FromValue is not implemented for u8
    assert_next(values, 1u16)?;
    assert_next(values, 1u32)?;
    assert_next(values, 1u64)?;
    assert_next(values, -2i8)?;
    assert_next(values, -2i16)?;
    assert_next(values, -2i32)?;
    assert_next(values, -2i64)?;
    assert_next(values, -1024.125f32)?;
    assert_next(values, -1024.125f64)?;
    assert_next(values, String::from(""))?;
    assert_next(values, String::from("foo"))?;
    assert_next(values, String::from("hello"))?;
    assert_next(values, String::from("nushell"))?;
    assert_next(values, String::from("love you"))?;
    assert_next(values, Vec::<u8>::from_iter([0xf0, 0xff, 0x00]))?;
    assert_next(values, Vec::<u8>::from_iter([0xde, 0xad, 0xbe, 0xef]))?;
    assert_next(values, Vec::<u8>::from_iter([0xc0, 0xff, 0xee, 0xff, 0xee]))?;
    assert_next(values, (true, -2))?;
    assert_next(values, (34, 1, ()))?;
    assert_next(values, (-1024.125, String::from("foo")))?;
    assert_next(
        values,
        HashMap::from_iter([
            (String::from("foo"), Value::test_int(-2)),
            (String::from("bar"), Value::test_string("hello")),
        ]),
    )?;
    assert_next(
        values,
        HashMap::from_iter([(String::from("hello"), Value::test_bool(true))]),
    )?;
    assert_next(
        values,
        HashMap::from_iter([
            (String::from("nushell"), String::from("rocks")),
            (String::from("foo"), String::from("bar")),
            (String::from("hello"), String::from("world")),
        ]),
    )?;
    assert_next(
        values,
        DateTime::parse_from_rfc3339("1970-01-01T00:00:01+00:00").expect("valid datetime format"),
    )?;
    assert_next(
        values,
        DateTime::parse_from_rfc3339("1970-01-01T00:00:01.100+00:00")
            .expect("valid datetime format"),
    )?;
    assert_next(
        values,
        DateTime::parse_from_rfc3339("1970-01-01T00:00:01.100+00:00")
            .expect("valid datetime format"),
    )?;
    assert!(values.next().is_none());

    Ok(())
}

#[test]
fn sample_roundtrip() -> Result {
    let path_to_sample_nuon = nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join("sample.nuon");

    let sample_nuon =
        std::fs::read_to_string(&path_to_sample_nuon).expect("failed to open sample.nuon");

    let sample_value =
        nuon::from_nuon(&sample_nuon, None).expect("failed to deserialize sample.nuon");

    let outcome: Value = test().run_with_data("to msgpack | from msgpack", sample_value.clone())?;
    assert_eq!(sample_value, outcome);
    Ok(())
}

#[test]
fn objects() -> Result {
    let value: (HashMap<String, String>, String) = msgpack_test_with_opts("objects", "--objects")?;
    assert_eq!(value.0["nushell"], "rocks");
    assert_eq!(value.0.len(), 1);
    assert_eq!(value.1, "seriously");
    Ok(())
}

#[test]
fn max_depth() -> Result {
    let shell_error = msgpack_test("max-depth").expect_error()?;
    let msg = shell_error.generic_msg()?;
    assert!(msg.contains("exceeded depth limit"));
    Ok(())
}

#[test]
fn non_utf8() -> Result {
    let shell_error = msgpack_test("non-utf8").expect_error()?;
    assert!(matches!(shell_error, ShellError::NonUtf8Custom { .. }));
    Ok(())
}

#[test]
fn empty() -> Result {
    let shell_error = msgpack_test("empty").expect_error()?;
    let msg = shell_error.generic_msg()?;
    assert_eq!(msg, "failed to fill whole buffer");
    Ok(())
}

#[test]
fn eof() -> Result {
    let shell_error = msgpack_test("eof").expect_error()?;
    let msg = shell_error.generic_msg()?;
    assert_eq!(msg, "failed to fill whole buffer");
    Ok(())
}

#[test]
fn after_eof() -> Result {
    let shell_error = msgpack_test("after-eof").expect_error()?;
    let error = shell_error.generic_error()?;
    assert_eq!(error, "Additional data after end of MessagePack object");
    Ok(())
}

#[test]
fn reserved() -> Result {
    let shell_error = msgpack_test("reserved").expect_error()?;
    let msg = shell_error.generic_msg()?;
    assert!(msg.contains("Reserved"));
    Ok(())
}

#[test]
fn u64_too_large() -> Result {
    let shell_error = msgpack_test("u64-too-large").expect_error()?;
    let error = shell_error.generic_error()?;
    assert_eq!(error, "MessagePack integer too big for Nushell");
    Ok(())
}

#[test]
fn non_string_map_key() -> Result {
    let shell_error = msgpack_test("non-string-map-key").expect_error()?;
    let msg = shell_error.generic_msg()?;
    assert!(msg.contains("string key"));
    Ok(())
}

#[test]
fn timestamp_wrong_length() -> Result {
    let shell_error = msgpack_test("timestamp-wrong-length").expect_error()?;
    let error = shell_error.generic_error()?;
    assert_eq!(error, "Unknown MessagePack extension");
    Ok(())
}

#[test]
fn other_extension_type() -> Result {
    let shell_error = msgpack_test("other-extension-type").expect_error()?;
    let error = shell_error.generic_error()?;
    assert_eq!(error, "Unknown MessagePack extension");
    Ok(())
}
