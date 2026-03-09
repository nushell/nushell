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

    let code = format!(
        "open '{}' | to msgpackz | from msgpackz | to nuon --indent 4",
        path_to_sample_nuon.display()
    );

    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome.replace("\r\n", "\n").trim(),
        sample_nuon.replace("\r\n", "\n").trim()
    );
    Ok(())
}
