use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExportModule;

impl Command for ExportModule {
    fn name(&self) -> &str {
        "export module"
    }

    fn usage(&self) -> &str {
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

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
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

    fn examples(&self) -> Vec<Example> {
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
