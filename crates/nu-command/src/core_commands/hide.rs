use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Value};

#[derive(Clone)]
pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide")
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
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
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new(call.head))
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
            Example {
                description: "Hide an environment variable",
                example: r#"let-env HZ_ENV_ABC = 1; hide HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name"#,
                result: Some(Value::boolean(false, Span::test_data())),
            },
        ]
    }
}
