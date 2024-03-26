use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Keybindings;

impl Command for Keybindings {
    fn name(&self) -> &str {
        "keybindings"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Keybindings related commands."
    }

    fn extra_usage(&self) -> &str {
        r#"You must use one of the following subcommands. Using this command as-is will only produce this help message.

For more information on input and keybindings, check:
  https://www.nushell.sh/book/line_editor.html"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shortcut", "hotkey"]
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
                &Keybindings.signature(),
                &Keybindings.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
