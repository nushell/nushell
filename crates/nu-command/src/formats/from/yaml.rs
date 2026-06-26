use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, casing::Casing};

#[derive(Clone)]
pub struct FromYamlLike(&'static str);
pub const FROM_YAML: FromYamlLike = FromYamlLike("from yaml");
pub const FROM_YML: FromYamlLike = FromYamlLike("from yml");

impl Command for FromYamlLike {
    fn name(&self) -> &str {
        self.0
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
            .named(
                "spec",
                SyntaxShape::String,
                "YAML spec version ('1.1' (default) or '1.2').",
                None,
            )
            .named(
                "multiple",
                SyntaxShape::String,
                "Handle multiple documents ('auto', 'list', 'single').",
                None,
            )
            .switch("ignore-tags", "Ignore any tags", None)
    }

    fn description(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: match self.name() {
                    "from yaml" => "'a: 1' | from yaml",
                    "from yml" => "'a: 1' | from yml",
                    _ => unreachable!("only implemented for `yaml` and `yml`"),
                },
                description: "Converts yaml formatted string to table.",
                result: Some(test_record! {
                    "a" => 1
                }),
            },
            Example {
                example: match self.name() {
                    "from yaml" => "'[ a: 1, b: [1, 2] ]' | from yaml",
                    "from yml" => "'[ a: 1, b: [1, 2] ]' | from yml",
                    _ => unreachable!("only implemented for `yaml` and `yml`"),
                },
                description: "Converts yaml formatted string to table.",
                result: Some(Value::test_list(vec![
                    test_record! { "a" => 1 },
                    test_record! { "b" => [1, 2] },
                ])),
            },
            Example {
                example: match self.name() {
                    "from yaml" => "'!cell-path $1.abc?.def!' | from yaml",
                    "from yml" => "'!cell-path $1.abc?.def!' | from yml",
                    _ => unreachable!("only implemented for `yaml` and `yml`"),
                },
                description: "Convert nushell values from yaml.",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![
                        PathMember::test_int(1, false),
                        PathMember::test_string("abc", true, Casing::Sensitive),
                        PathMember::test_string("def", false, Casing::Insensitive),
                    ],
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input
            .take_metadata()
            .map(|meta| meta.with_content_type(None));
        let (yaml, yaml_span, ..) = input.collect_string_strict(call.head)?;
        let yaml = yaml.as_str().into_spanned(yaml_span);
        let spec = call.get_flag(engine_state, stack, "spec")?;
        let multiple = call.get_flag(engine_state, stack, "multiple")?;
        let ignore_tags = call.has_flag(engine_state, stack, "ignore-tags")?;
        let options = nu_heavy_utils::yaml::ParseOptions::default()
            .spec(spec.unwrap_or_default())
            .multiple(multiple.unwrap_or_default())
            .ignore_tags(ignore_tags);
        nu_heavy_utils::yaml::parse(yaml, call.head, options)
            .map(|val| PipelineData::value(val, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_test_support::prelude::{Result, *};

    #[test]
    fn test_examples() -> Result {
        test().examples(FROM_YAML)?;
        test().examples(FROM_YML)?;
        Ok(())
    }

    #[test]
    fn test_content_type_metadata() -> Result {
        let code = r#"
          "a: 1\nb: 2"
          | metadata set --content-type 'application/yaml' --path-columns [name]
          | from yaml
          | metadata
          | reject span
        "#;

        test().run(code).expect_value_eq(test_record! {
            "path_columns" => ["name"]
        })
    }
}
