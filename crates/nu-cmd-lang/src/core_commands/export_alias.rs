use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportAlias;

impl Command for ExportAlias {
    fn name(&self) -> &str {
        "export alias"
    }

    fn usage(&self) -> &str {
        "Alias a command (with optional flags) to a new name and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export alias")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "Name of the alias.")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "Equals sign followed by value.",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["abbr", "aka", "fn", "func", "function"]
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
            description: "Alias ll to ls -l and export it from a module",
            example: "module spam { export alias ll = ls -l }",
            result: Some(Value::nothing(Span::test_data())),
        }]
    }
}
