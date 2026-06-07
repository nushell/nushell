use nu_protocol::{FromValue, IntoSpanned, Span};

const FIXTURE: &str = include_str!("../../../tests/fixtures/formats/sample.yaml");

fn main() {
    let span = Span::test_data();
    let options = nu_heavy_utils::yaml::ParseOptions::default();
    let yaml = FIXTURE.into_spanned(span);
    let parsed = nu_heavy_utils::yaml::parse(yaml, span, &options).unwrap();
    let serializable = nu_json::Value::from_value(parsed).unwrap();
    let json = serde_json::to_string_pretty(&serializable).unwrap();
    println!("{json}");
}
