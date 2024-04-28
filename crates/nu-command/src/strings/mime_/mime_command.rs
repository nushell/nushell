use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct MimeCommand;

impl Command for MimeCommand {
    fn name(&self) -> &str {
        "mime"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Strings)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["mime", "guess"]
    }

    fn usage(&self) -> &str {
        "Various commands for working with MIME/Media Types."
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(
                &MimeCommand.signature(),
                &MimeCommand.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
