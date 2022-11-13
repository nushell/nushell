use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, Type, Value,
};

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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &ExportCommand.signature(),
                &ExportCommand.examples(),
                engine_state,
                stack,
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Export a definition from a module",
            example: r#"module utils { export def my-command [] { "hello" } }; use utils my-command; my-command"#,
            result: Some(Value::String {
                val: "hello".to_string(),
                span: Span::test_data(),
            }),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
