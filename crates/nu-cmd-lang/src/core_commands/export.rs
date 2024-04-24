use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct ExportCommand;

impl Command for ExportCommand {
    fn name(&self) -> &str {
        "export"
    }

    fn signature(&self) -> Signature {
        Signature::build("export")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Export definitions or environment variables from a module."
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(
                &ExportCommand.signature(),
                &ExportCommand.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Export a definition from a module",
            example: r#"module utils { export def my-command [] { "hello" } }; use utils my-command; my-command"#,
            result: Some(Value::test_string("hello")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
