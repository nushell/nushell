use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Abbr;

impl Command for Abbr {
    fn name(&self) -> &str {
        "abbr"
    }

    fn description(&self) -> &str {
        "Create an abbreviation for a command (with optional flags)."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("abbr")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "Name of the abbreviation.")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "Equals sign followed by the expanded command value.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For more details, see:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["alias", "shortcut", "shorthand", "expansion"]
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Create an abbreviation 'll' that expands to 'ls -l'",
            example: "abbr ll = ls -l",
            result: Some(Value::nothing(Span::test_data())),
        }]
    }
}
