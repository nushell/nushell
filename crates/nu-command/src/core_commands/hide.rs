use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("module", SyntaxShape::String, "Module or module file")
            .optional(
                "members",
                SyntaxShape::Any,
                "Which members of the module to import",
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Hide definitions in the current scope"
    }

    fn extra_usage(&self) -> &str {
        r#"Definitions are hidden by priority: First aliases, then custom commands.

This command is a parser keyword. For details, check:
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
        vec![
            Example {
                description: "Hide the alias just defined",
                example: r#"alias lll = ls -l; hide lll"#,
                result: None,
            },
            Example {
                description: "Hide a custom command",
                example: r#"def say-hi [] { echo 'Hi!' }; hide say-hi"#,
                result: None,
            },
        ]
    }
}
