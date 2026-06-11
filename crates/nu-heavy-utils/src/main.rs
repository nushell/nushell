use nu_protocol::{IntoSpanned, Span};

const FIXTURE: &str = include_str!("../../../tests/fixtures/formats/sample.yaml");

fn main() {
    let span = Span::test_data();
    let options = nu_heavy_utils::yaml::ParseOptions::default();
    let yaml = FIXTURE.into_spanned(span);
    let parsed = nu_heavy_utils::yaml::parse(yaml, span, &options).unwrap();
    let parsed = dbg!(parsed);

    let options = nu_heavy_utils::yaml::SerializeOptions::default();
    let serialized = nu_heavy_utils::yaml::serialize(&parsed, span, &options).unwrap();
    println!("{serialized}");
}
