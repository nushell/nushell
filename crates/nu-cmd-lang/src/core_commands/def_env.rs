use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct DefEnv;

impl Command for DefEnv {
    fn name(&self) -> &str {
        "def-env"
    }

    fn usage(&self) -> &str {
        "Define a custom command, which participates in the caller environment."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("def-env")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("block", SyntaxShape::Block, "body of the definition")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html
"#
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
            description: "Set environment variable by call a custom command",
            example: r#"def-env foo [] { $env.BAR = "BAZ" }; foo; $env.BAR"#,
            result: Some(Value::test_string("BAZ")),
        }]
    }
}
