use nu_engine::command_prelude::*;
use nu_heavy_utils::yaml::{NonRoundtrip, SerializeOptions};

#[derive(Clone)]
pub struct ToYamlLike(&'static str);
pub const TO_YAML: ToYamlLike = ToYamlLike("to yaml");
pub const TO_YML: ToYamlLike = ToYamlLike("to yml");

impl Command for ToYamlLike {
    fn name(&self) -> &str {
        self.0
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized.",
                Some('s'),
            )
            .param(
                Flag::new("non-roundtrip")
                    .arg(SyntaxShape::String)
                    .desc("How to handle values that are non-roundtrippable.")
                    .completion(Completion::new_list(&["error", "null", "lossy"])),
            )
            .param(
                Flag::new("spec")
                    .arg(SyntaxShape::String)
                    .desc("YAML spec version ('1.1' or '1.2' (default)).")
                    .completion(Completion::new_list(&["1.1", "1.2"])),
            )
            .switch("add-directives", "Add YAML document directives.", Some('d'))
            .switch(
                "multiple",
                "Given a list, serialize a multi document stream.",
                Some('m'),
            )
            .named(
                "indent",
                SyntaxShape::Int,
                "Configure the indent.",
                Some('i'),
            )
            .switch(
                "compact-list-indent",
                "Emit lists with a more compact indentation style.",
                None,
            )
            .param(
                Flag::new("quote")
                    .short('q')
                    .arg(SyntaxShape::String)
                    .desc("String quote style ('auto' (default), 'single' or 'double')")
                    .completion(Completion::new_list(&["auto", "single", "double"])),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into .yaml/.yml text."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a YAML string representing the contents of this table.",
                example: match self.name() {
                    "to yaml" => r#"[[foo bar]; ["1" "2"]] | to yaml"#,
                    "to yml" => r#"[[foo bar]; ["1" "2"]] | to yml"#,
                    _ => unreachable!("only implemented for `yaml` and `yml`"),
                },
                result: Some(Value::test_string("- foo: \"1\"\n  bar: \"2\"\n")),
            },
            Example {
                description: "Convert a nushell specific type into YAML.",
                example: match self.name() {
                    "to yaml" => "$.1.abc | to yaml",
                    "to yml" => "$.1.abc | to yml",
                    _ => unreachable!("only implemented for `yaml` and `yml`"),
                },
                result: Some(Value::test_string("!cell-path $.1.abc\n")),
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
            .unwrap_or_default()
            .with_content_type(Some(String::from("application/yaml")));
        let value = input.into_value(call.head)?;
        let spec = call.get_flag(engine_state, stack, "spec")?;
        let add_directives = call.has_flag(engine_state, stack, "add-directives")?;
        let multiple = call.has_flag(engine_state, stack, "multiple")?;
        let indent = call.get_flag(engine_state, stack, "indent")?;
        let compact_list_indent = call.get_flag(engine_state, stack, "compact-list-indent")?;
        let quote_style = call.get_flag(engine_state, stack, "quote")?;
        let non_roundtrip =
            call.get_flag::<Spanned<String>>(engine_state, stack, "non-roundtrip")?;
        let non_roundtrip = match (
            call.has_flag(engine_state, stack, "serialize")?,
            non_roundtrip.as_ref().map(|nr| nr.item.as_ref()),
        ) {
            // matching the spanned is way less comprehendible here, so we expect spans instead
            (false, None | Some("error")) => NonRoundtrip::Error,
            (true, None | Some("lossy")) => NonRoundtrip::Lossy {
                engine_state: Box::new(engine_state.clone()),
            },
            (false, Some("null")) => NonRoundtrip::Null,
            (false, Some(_)) => {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "expected `error`, `null` or `lossy`".into(),
                    span: non_roundtrip.expect("non_roundtrip is some").span,
                });
            }
            (true, Some(_)) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "this is a shorthand to".into(),
                    left_span: call
                        .get_flag_span(stack, "serialize")
                        .expect("serialize is some"),
                    right_message: "this with `lossy`".into(),
                    right_span: non_roundtrip.expect("non_roundtrip is some").span,
                });
            }
        };

        match call.has_flag(engine_state, stack, "serialize")? {
            true => NonRoundtrip::Lossy {
                engine_state: Box::new(engine_state.clone()),
            },
            false => NonRoundtrip::Null,
        };

        let defaults = SerializeOptions::default();
        let options = SerializeOptions::default()
            .with_spec(spec.unwrap_or(defaults.spec))
            .with_non_roundtrip(non_roundtrip)
            .with_add_directives(add_directives)
            .with_multiple(multiple)
            .with_indent(indent.unwrap_or(defaults.indent))
            .with_compact_list_indent(compact_list_indent.unwrap_or(defaults.compact_list_indent))
            .with_quote_style(quote_style.unwrap_or(defaults.quote_style));

        nu_heavy_utils::yaml::serialize(&value, call.head, options)
            .map(|s| PipelineData::value(Value::string(s, call.head), metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_test_support::prelude::{Result, *};

    #[test]
    fn test_examples() -> Result {
        test().examples(TO_YAML)?;
        test().examples(TO_YML)?;
        Ok(())
    }

    #[test]
    fn test_content_type_metadata() -> Result {
        let code = "
            {a: 1, b: 2}
            | to yaml
            | metadata
            | get content_type
        ";

        test().run(code).expect_value_eq("application/yaml")
    }
}
