use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExternWrapped;

impl Command for ExternWrapped {
    fn name(&self) -> &str {
        "extern-wrapped"
    }

    fn usage(&self) -> &str {
        "Define a signature for an external command with a custom code block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("extern-wrapped")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("body", SyntaxShape::Block, "wrapper code block")
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError(
                "Deprecated command".into(),
                "`extern-wrapped` is deprecated and will be removed in 0.88.".into(),
                Some(call.head),
                Some("Use `def --wrapped` instead".into()),
                vec![],
            ),
        );
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Define a custom wrapper for an external command",
            example: r#"extern-wrapped my-echo [...rest] { echo $rest }; my-echo spam"#,
            result: Some(Value::test_list(vec![Value::test_string("spam")])),
        }]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ExternWrapped;
        use crate::test_examples;
        test_examples(ExternWrapped {})
    }
}
