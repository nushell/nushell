use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct Module;

impl Command for Module {
    fn name(&self) -> &str {
        "module"
    }

    fn usage(&self) -> &str {
        "Define a custom module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("module")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("module_name", SyntaxShape::String, "module name")
            .required("block", SyntaxShape::Block, "body of the module")
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
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Define a custom command in a module and call it",
                example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
                result: Some(Value::String {
                    val: "foo".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Define an environment variable in a module",
                example: r#"module foo { export-env { let-env FOO = "BAZ" } }; use foo; $env.FOO"#,
                result: Some(Value::String {
                    val: "BAZ".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Define a custom command that participates in the environment in a module and call it",
                example: r#"module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
                result: Some(Value::String {
                    val: "BAZ".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}
