use nu_protocol::{IntoSpanned, Span};

const FIXTURE: &str = include_str!("../../../tests/fixtures/formats/sample.yaml");

fn main() {
    let span = Span::test_data();
    let options = nu_heavy_utils::yaml::ParseOptions::default();
    let yaml = FIXTURE.into_spanned(span);
    let parsed = nu_heavy_utils::yaml::parse(yaml, span, &options).unwrap();
    dbg!(parsed);
}
