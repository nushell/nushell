use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportModule;

impl Command for ExportModule {
    fn name(&self) -> &str {
        "export module"
    }

    fn description(&self) -> &str {
        "Export a custom module from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export module")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("module", SyntaxShape::String, "Module name or module path.")
            .optional(
                "block",
                SyntaxShape::Block,
                "Body of the module if 'module' parameter is not a path.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Define a custom command in a submodule of a module and call it",
            example: r#"module spam {
        export module eggs {
            export def foo [] { "foo" }
        }
    }
    use spam eggs
    eggs foo"#,
            result: Some(Value::test_string("foo")),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ExportModule {})
    }
}
