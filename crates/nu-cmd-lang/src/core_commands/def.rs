use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct Def;

impl Command for Def {
    fn name(&self) -> &str {
        "def"
    }

    fn usage(&self) -> &str {
        "Define a custom command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("def")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("body", SyntaxShape::Closure(None), "body of the definition")
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
        vec![
            Example {
                description: "Define a command and run it",
                example: r#"def say-hi [] { echo 'hi' }; say-hi"#,
                result: Some(SpannedValue::test_string("hi")),
            },
            Example {
                description: "Define a command and run it with parameter(s)",
                example: r#"def say-sth [sth: string] { echo $sth }; say-sth hi"#,
                result: Some(SpannedValue::test_string("hi")),
            },
        ]
    }
}
