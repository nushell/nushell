use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .input_output_types(vec![
                (Type::Date, Type::String),
                (Type::String, Type::String),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "lists strftime cheatsheet", Some('l'))
            .optional(
                "format string",
                SyntaxShape::String,
                "the desired date format",
            )
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Format a given date using a format string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fmt", "strftime"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::DeprecatedCommand(
            self.name().to_string(),
            "format date".to_owned(),
            call.head,
        ))
    }
}
