use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct ExportAlias;

impl Command for ExportAlias {
    fn name(&self) -> &str {
        "export alias"
    }

    fn usage(&self) -> &str {
        "Define an alias and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export alias")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "name of the alias")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            )
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
            description: "export an alias of ll to ls -l, from a module",
            example: "export alias ll = ls -l",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["aka", "abbr", "module"]
    }
}
