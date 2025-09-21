use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportExtern;

impl Command for ExportExtern {
    fn name(&self) -> &str {
        "export extern"
    }

    fn description(&self) -> &str {
        "Define an extern and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export extern")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "Definition name.")
            .required("params", SyntaxShape::Signature, "Parameters.")
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
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
            description: "Export the signature for an external command",
            example: r#"export extern echo [text: string]"#,
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["signature", "module", "declare"]
    }
}
