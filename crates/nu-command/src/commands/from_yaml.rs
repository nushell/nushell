use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromYAML;

#[async_trait]
impl WholeStreamCommand for FromYAML {
    fn name(&self) -> &str {
        "from yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from yaml")
    }

    fn usage(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_yaml(args).await
    }
}

pub struct FromYML;

#[async_trait]
impl WholeStreamCommand for FromYML {
    fn name(&self) -> &str {
        "from yml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from yml")
    }

    fn usage(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_yaml(args).await
    }
}

fn convert_yaml_value_to_nu_value(
    v: &serde_yaml::Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let span = tag.span;

    let err_not_compatible_number = ShellError::labeled_error(
        "Expected a compatible number",
        "expected a compatible number",
        &tag,
    );
    Ok(match v {
        serde_yaml::Value::Bool(b) => UntaggedValue::boolean(*b).into_value(tag),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            UntaggedValue::int(n.as_i64().ok_or(err_not_compatible_number)?).into_value(tag)
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            UntaggedValue::decimal_from_float(n.as_f64().ok_or(err_not_compatible_number)?, span)
                .into_value(tag)
        }
        serde_yaml::Value::String(s) => UntaggedValue::string(s).into_value(tag),
        serde_yaml::Value::Sequence(a) => {
            let result: Result<Vec<Value>, ShellError> = a
                .iter()
                .map(|x| convert_yaml_value_to_nu_value(x, &tag))
                .collect();
            UntaggedValue::Table(result?).into_value(tag)
        }
        serde_yaml::Value::Mapping(t) => {
            let mut collected = TaggedDictBuilder::new(&tag);

            for (k, v) in t.iter() {
                // A ShellError that we re-use multiple times in the Mapping scenario
                let err_unexpected_map = ShellError::labeled_error(
                    format!("Unexpected YAML:\nKey: {:?}\nValue: {:?}", k, v),
                    "unexpected",
                    tag.clone(),
                );
                match (k, v) {
                    (serde_yaml::Value::String(k), _) => {
                        collected.insert_value(k.clone(), convert_yaml_value_to_nu_value(v, &tag)?);
                    }
                    // Hard-code fix for cases where "v" is a string without quotations with double curly braces
                    // e.g. k = value
                    // value: {{ something }}
                    // Strangely, serde_yaml returns
                    // "value" -> Mapping(Mapping { map: {Mapping(Mapping { map: {String("something"): Null} }): Null} })
                    (serde_yaml::Value::Mapping(m), serde_yaml::Value::Null) => {
                        return m
                            .iter()
                            .take(1)
                            .collect_vec()
                            .first()
                            .and_then(|e| match e {
                                (serde_yaml::Value::String(s), serde_yaml::Value::Null) => Some(
                                    UntaggedValue::string("{{ ".to_owned() + &s + " }}")
                                        .into_value(tag),
                                ),
                                _ => None,
                            })
                            .ok_or(err_unexpected_map);
                    }
                    (_, _) => {
                        return Err(err_unexpected_map);
                    }
                }
            }

            collected.into_value()
        }
        serde_yaml::Value::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(tag),
        x => unimplemented!("Unsupported yaml case: {:?}", x),
    })
}

pub fn from_yaml_string_to_value(s: String, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).map_err(|x| {
        ShellError::labeled_error(
            format!("Could not load yaml: {}", x),
            "could not load yaml from text",
            &tag,
        )
    })?;
    convert_yaml_value_to_nu_value(&v, tag)
}

async fn from_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;

    let concat_string = input.collect_string(tag.clone()).await?;

    match from_yaml_string_to_value(concat_string.item, tag.clone()) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(futures::stream::iter(list).to_output_stream()),
            x => Ok(OutputStream::one(x)),
        },
        Err(_) => Err(ShellError::labeled_error_with_secondary(
            "Could not parse as YAML",
            "input cannot be parsed as YAML",
            &tag,
            "value originates from here",
            &concat_string.tag,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::*;
    use nu_protocol::row;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromYAML {})
    }

    #[test]
    fn test_problematic_yaml() {
        struct TestCase {
            description: &'static str,
            input: &'static str,
            expected: Result<Value, ShellError>,
        }
        let tt: Vec<TestCase> = vec![
            TestCase {
                description: "Double Curly Braces With Quotes",
                input: r#"value: "{{ something }}""#,
                expected: Ok(row!["value".to_owned() => string("{{ something }}")]),
            },
            TestCase {
                description: "Double Curly Braces Without Quotes",
                input: r#"value: {{ something }}"#,
                expected: Ok(row!["value".to_owned() => string("{{ something }}")]),
            },
        ];
        for tc in tt.into_iter() {
            let actual = from_yaml_string_to_value(tc.input.to_owned(), Tag::default());
            if actual.is_err() {
                assert!(
                    tc.expected.is_err(),
                    "actual is Err for test:\nTest Description {}\nErr: {:?}",
                    tc.description,
                    actual
                );
            } else {
                assert_eq!(actual, tc.expected, "{}", tc.description);
            }
        }
    }
}
