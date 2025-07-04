use nu_test_support::{nu, playground::Playground};
use pretty_assertions::assert_eq;

fn msgpack_test(fixture_name: &str, commands: Option<&str>) -> nu_test_support::Outcome {
    let path_to_generate_nu = nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join("generate.nu");

    let mut outcome = None;
    Playground::setup(&format!("msgpack test {fixture_name}"), |dirs, _| {
        assert!(
            nu!(
                cwd: dirs.test(),
                format!(
                    "nu -n '{}' '{}'",
                    path_to_generate_nu.display(),
                    fixture_name
                ),
            )
            .status
            .success()
        );

        outcome = Some(nu!(
            cwd: dirs.test(),
            collapse_output: false,
            commands.map(|c| c.to_owned()).unwrap_or_else(|| format!("open {fixture_name}.msgpack"))
        ));
    });
    outcome.expect("failed to get outcome")
}

fn msgpack_nuon_test(fixture_name: &str, opts: &str) {
    let path_to_nuon = nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join(format!("{fixture_name}.nuon"));

    let sample_nuon = std::fs::read_to_string(path_to_nuon).expect("failed to open nuon file");

    let outcome = msgpack_test(
        fixture_name,
        Some(&format!(
            "open --raw {fixture_name}.msgpack | from msgpack {opts} | to nuon --indent 4"
        )),
    );

    assert!(outcome.status.success());
    assert!(outcome.err.is_empty());
    assert_eq!(
        sample_nuon.replace("\r\n", "\n"),
        outcome.out.replace("\r\n", "\n")
    );
}

#[test]
fn sample() {
    msgpack_nuon_test("sample", "");
}

#[test]
fn sample_roundtrip() {
    let path_to_sample_nuon = nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join("sample.nuon");

    let sample_nuon =
        std::fs::read_to_string(&path_to_sample_nuon).expect("failed to open sample.nuon");

    let outcome = nu!(
        collapse_output: false,
        format!(
            "open '{}' | to msgpack | from msgpack | to nuon --indent 4",
            path_to_sample_nuon.display()
        )
    );

    assert!(outcome.status.success());
    assert!(outcome.err.is_empty());
    assert_eq!(
        sample_nuon.replace("\r\n", "\n"),
        outcome.out.replace("\r\n", "\n")
    );
}

#[test]
fn objects() {
    msgpack_nuon_test("objects", "--objects");
}

#[test]
fn max_depth() {
    let outcome = msgpack_test("max-depth", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("exceeded depth limit"));
}

#[test]
fn non_utf8() {
    let outcome = msgpack_test("non-utf8", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("utf-8"));
}

#[test]
fn empty() {
    let outcome = msgpack_test("empty", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("fill whole buffer"));
}

#[test]
fn eof() {
    let outcome = msgpack_test("eof", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("fill whole buffer"));
}

#[test]
fn after_eof() {
    let outcome = msgpack_test("after-eof", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("after end of"));
}

#[test]
fn reserved() {
    let outcome = msgpack_test("reserved", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("Reserved"));
}

#[test]
fn u64_too_large() {
    let outcome = msgpack_test("u64-too-large", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("integer too big"));
}

#[test]
fn non_string_map_key() {
    let outcome = msgpack_test("non-string-map-key", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("string key"));
}

#[test]
fn timestamp_wrong_length() {
    let outcome = msgpack_test("timestamp-wrong-length", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("Unknown MessagePack extension"));
}

#[test]
fn other_extension_type() {
    let outcome = msgpack_test("other-extension-type", None);
    assert!(!outcome.status.success());
    assert!(outcome.err.contains("Unknown MessagePack extension"));
}
