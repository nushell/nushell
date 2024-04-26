use nu_test_support::nu;
use pretty_assertions::assert_eq;

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
            "open '{}' | to msgpackz | from msgpackz | to nuon --indent 4",
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
