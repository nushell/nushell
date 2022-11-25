use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Extern;

impl Command for Extern {
    fn name(&self) -> &str {
        "extern"
    }

    fn usage(&self) -> &str {
        "Define a signature for an external command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("extern")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
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
        vec![Example {
            description: "Write a signature for an external command",
            example: r#"extern echo [text: string]"#,
            result: None,
        }]
    }
}
