use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Value};

#[derive(Clone)]
pub struct ExportEnv;

impl Command for ExportEnv {
    fn name(&self) -> &str {
        "export env"
    }

    fn usage(&self) -> &str {
        "Export a block from a module that will be evaluated as an environment variable when imported."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export env")
            .required(
                "name",
                SyntaxShape::String,
                "name of the environment variable",
            )
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the environment variable definition",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nushell.html"#
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
        //TODO: Add the env to stack
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Import and evaluate environment variable from a module",
            example: r#"module foo { export env FOO_ENV { "BAZ" } }; use foo FOO_ENV; $env.FOO_ENV"#,
            result: Some(Value::String {
                val: "BAZ".to_string(),
                span: Span::test_data(),
            }),
        }]
    }
}
