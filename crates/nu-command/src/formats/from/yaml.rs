use indexmap::IndexMap;
use itertools::Itertools;
use nu_engine::command_prelude::*;
use serde::de::Deserialize;

#[derive(Clone)]
pub struct FromYaml;

impl Command for FromYaml {
    fn name(&self) -> &str {
        "from yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from yaml")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    fn examples(&self) -> Vec<Example> {
        get_examples()
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        from_yaml(input, head)
    }
}

#[derive(Clone)]
pub struct FromYml;

impl Command for FromYml {
    fn name(&self) -> &str {
        "from yml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from yml")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        from_yaml(input, head)
    }

    fn examples(&self) -> Vec<Example> {
        get_examples()
    }
}

fn convert_yaml_value_to_nu_value(
    v: &serde_yaml::Value,
    span: Span,
    val_span: Span,
) -> Result<Value, ShellError> {
    let err_not_compatible_number = ShellError::UnsupportedInput {
        msg: "Expected a nu-compatible number in YAML input".to_string(),
        input: "value originates from here".into(),
        msg_span: span,
        input_span: val_span,
    };
    Ok(match v {
        serde_yaml::Value::Bool(b) => Value::bool(*b, span),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            Value::int(n.as_i64().ok_or(err_not_compatible_number)?, span)
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            Value::float(n.as_f64().ok_or(err_not_compatible_number)?, span)
        }
        serde_yaml::Value::String(s) => Value::string(s.to_string(), span),
        serde_yaml::Value::Sequence(a) => {
            let result: Result<Vec<Value>, ShellError> = a
                .iter()
                .map(|x| convert_yaml_value_to_nu_value(x, span, val_span))
                .collect();
            Value::list(result?, span)
        }
        serde_yaml::Value::Mapping(t) => {
            // Using an IndexMap ensures consistent ordering
            let mut collected = IndexMap::new();

            for (k, v) in t {
                // A ShellError that we re-use multiple times in the Mapping scenario
                let err_unexpected_map = ShellError::UnsupportedInput {
                    msg: format!("Unexpected YAML:\nKey: {k:?}\nValue: {v:?}"),
                    input: "value originates from here".into(),
                    msg_span: span,
                    input_span: val_span,
                };
                match (k, v) {
                    (serde_yaml::Value::Number(k), _) => {
                        collected.insert(
                            k.to_string(),
                            convert_yaml_value_to_nu_value(v, span, val_span)?,
                        );
                    }
                    (serde_yaml::Value::Bool(k), _) => {
                        collected.insert(
                            k.to_string(),
                            convert_yaml_value_to_nu_value(v, span, val_span)?,
                        );
                    }
                    (serde_yaml::Value::String(k), _) => {
                        collected.insert(
                            k.clone(),
                            convert_yaml_value_to_nu_value(v, span, val_span)?,
                        );
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
                                (serde_yaml::Value::String(s), serde_yaml::Value::Null) => {
                                    Some(Value::string("{{ ".to_owned() + s.as_str() + " }}", span))
                                }
                                _ => None,
                            })
                            .ok_or(err_unexpected_map);
                    }
                    (_, _) => {
                        return Err(err_unexpected_map);
                    }
                }
            }

            Value::record(collected.into_iter().collect(), span)
        }
        serde_yaml::Value::Tagged(t) => {
            let tag = &t.tag;

            match &t.value {
                serde_yaml::Value::String(s) => {
                    let val = format!("{tag} {s}").trim().to_string();
                    Value::string(val, span)
                }
                serde_yaml::Value::Number(n) => {
                    let val = format!("{tag} {n}").trim().to_string();
                    Value::string(val, span)
                }
                serde_yaml::Value::Bool(b) => {
                    let val = format!("{tag} {b}").trim().to_string();
                    Value::string(val, span)
                }
                serde_yaml::Value::Null => {
                    let val = format!("{tag}").trim().to_string();
                    Value::string(val, span)
                }
                v => convert_yaml_value_to_nu_value(v, span, val_span)?,
            }
        }
        serde_yaml::Value::Null => Value::nothing(span),
        x => unimplemented!("Unsupported YAML case: {:?}", x),
    })
}

pub fn from_yaml_string_to_value(s: &str, span: Span, val_span: Span) -> Result<Value, ShellError> {
    let mut documents = vec![];

    for document in serde_yaml::Deserializer::from_str(s) {
        let v: serde_yaml::Value =
            serde_yaml::Value::deserialize(document).map_err(|x| ShellError::UnsupportedInput {
                msg: format!("Could not load YAML: {x}"),
                input: "value originates from here".into(),
                msg_span: span,
                input_span: val_span,
            })?;
        documents.push(convert_yaml_value_to_nu_value(&v, span, val_span)?);
    }

    match documents.len() {
        0 => Ok(Value::nothing(span)),
        1 => Ok(documents.remove(0)),
        _ => Ok(Value::list(documents, span)),
    }
}

pub fn get_examples() -> Vec<Example<'static>> {
    vec![
        Example {
            example: "'a: 1' | from yaml",
            description: "Converts yaml formatted string to table",
            result: Some(Value::test_record(record! {
                "a" => Value::test_int(1),
            })),
        },
        Example {
            example: "'[ a: 1, b: [1, 2] ]' | from yaml",
            description: "Converts yaml formatted string to table",
            result: Some(Value::test_list(vec![
                Value::test_record(record! {
                    "a" => Value::test_int(1),
                }),
                Value::test_record(record! {
                    "b" => Value::test_list(
                        vec![Value::test_int(1), Value::test_int(2)],),
                }),
            ])),
        },
    ]
}

fn from_yaml(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(head)?;

    match from_yaml_string_to_value(&concat_string, head, span) {
        Ok(x) => {
            Ok(x.into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
        }
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod test {
    use crate::Reject;
    use crate::{Metadata, MetadataSet};

    use super::*;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;
    use nu_protocol::Config;

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
                expected: Ok(Value::test_record(record! {
                    "value" => Value::test_string("{{ something }}"),
                })),
            },
            TestCase {
                description: "Double Curly Braces Without Quotes",
                input: r#"value: {{ something }}"#,
                expected: Ok(Value::test_record(record! {
                    "value" => Value::test_string("{{ something }}"),
                })),
            },
        ];
        let config = Config::default();
        for tc in tt {
            let actual = from_yaml_string_to_value(tc.input, Span::test_data(), Span::test_data());
            if actual.is_err() {
                assert!(
                    tc.expected.is_err(),
                    "actual is Err for test:\nTest Description {}\nErr: {:?}",
                    tc.description,
                    actual
                );
            } else {
                assert_eq!(
                    actual.unwrap().to_expanded_string("", &config),
                    tc.expected.unwrap().to_expanded_string("", &config)
                );
            }
        }
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromYaml {})
    }

    #[test]
    fn test_consistent_mapping_ordering() {
        let test_yaml = "- a: b
  b: c
- a: g
  b: h";

        // Before the fix this test is verifying, the ordering of columns in the resulting
        // table was non-deterministic. It would take a few executions of the YAML conversion to
        // see this ordering difference. This loop should be far more than enough to catch a regression.
        for ii in 1..1000 {
            let actual = from_yaml_string_to_value(test_yaml, Span::test_data(), Span::test_data());

            let expected: Result<Value, ShellError> = Ok(Value::test_list(vec![
                Value::test_record(record! {
                    "a" => Value::test_string("b"),
                    "b" => Value::test_string("c"),
                }),
                Value::test_record(record! {
                    "a" => Value::test_string("g"),
                    "b" => Value::test_string("h"),
                }),
            ]));

            // Unfortunately the eq function for Value doesn't compare well enough to detect
            // ordering errors in List columns or values.

            assert!(actual.is_ok());
            let actual = actual.ok().unwrap();
            let expected = expected.ok().unwrap();

            let actual_vals = actual.as_list().unwrap();
            let expected_vals = expected.as_list().unwrap();
            assert_eq!(expected_vals.len(), actual_vals.len(), "iteration {ii}");

            for jj in 0..expected_vals.len() {
                let actual_record = actual_vals[jj].as_record().unwrap();
                let expected_record = expected_vals[jj].as_record().unwrap();

                let actual_columns = actual_record.columns();
                let expected_columns = expected_record.columns();
                assert!(
                    expected_columns.eq(actual_columns),
                    "record {jj}, iteration {ii}"
                );

                let actual_vals = actual_record.values();
                let expected_vals = expected_record.values();
                assert!(expected_vals.eq(actual_vals), "record {jj}, iteration {ii}")
            }
        }
    }

    #[test]
    fn test_convert_yaml_value_to_nu_value_for_tagged_values() {
        struct TestCase {
            input: &'static str,
            expected: Result<Value, ShellError>,
        }

        let test_cases: Vec<TestCase> = vec![
            TestCase {
                input: "Key: !Value ${TEST}-Test-role",
                expected: Ok(Value::test_record(record! {
                    "Key" => Value::test_string("!Value ${TEST}-Test-role"),
                })),
            },
            TestCase {
                input: "Key: !Value test-${TEST}",
                expected: Ok(Value::test_record(record! {
                    "Key" => Value::test_string("!Value test-${TEST}"),
                })),
            },
            TestCase {
                input: "Key: !Value",
                expected: Ok(Value::test_record(record! {
                    "Key" => Value::test_string("!Value"),
                })),
            },
            TestCase {
                input: "Key: !True",
                expected: Ok(Value::test_record(record! {
                    "Key" => Value::test_string("!True"),
                })),
            },
            TestCase {
                input: "Key: !123",
                expected: Ok(Value::test_record(record! {
                    "Key" => Value::test_string("!123"),
                })),
            },
        ];

        for test_case in test_cases {
            let doc = serde_yaml::Deserializer::from_str(test_case.input);
            let v: serde_yaml::Value = serde_yaml::Value::deserialize(doc.last().unwrap()).unwrap();
            let result = convert_yaml_value_to_nu_value(&v, Span::test_data(), Span::test_data());
            assert!(result.is_ok());
            assert!(result.ok().unwrap() == test_case.expected.ok().unwrap());
        }
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromYaml {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#""a: 1\nb: 2" | metadata set --content-type 'application/yaml' --datasource-ls | from yaml | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("source" => Value::test_string("ls"))),
            result.expect("There should be a result")
        )
    }
}
