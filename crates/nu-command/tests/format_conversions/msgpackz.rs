use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn sample_roundtrip() -> Result {
    let path_to_sample_nuon = nu_test_support::fs::fixtures()
        .join("formats")
        .join("msgpack")
        .join("sample.nuon");

    let sample_nuon =
        std::fs::read_to_string(&path_to_sample_nuon).expect("failed to open sample.nuon");

    let code = "open $in | to msgpackz | from msgpackz | to nuon --indent 4";

    let outcome: String = test().run_with_data(code, path_to_sample_nuon)?;
    assert_eq!(
        outcome.replace("\r\n", "\n").trim(),
        sample_nuon.replace("\r\n", "\n").trim()
    );
    Ok(())
}
