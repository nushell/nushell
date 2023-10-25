use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct ExportExternWrapped;

impl Command for ExportExternWrapped {
    fn name(&self) -> &str {
        "export extern-wrapped"
    }

    fn usage(&self) -> &str {
        "Define an extern with a custom code block and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export extern-wrapped")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("body", SyntaxShape::Block, "wrapper code block")
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError(
                "Deprecated command".into(),
                "`export extern-wrapped` is deprecated and will be removed in 0.88.".into(),
                Some(call.head),
                Some("Use `export def --wrapped` instead".into()),
                vec![],
            ),
        );
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Export the signature for an external command",
            example: r#"export extern-wrapped my-echo [...rest] { echo $rest }"#,
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["signature", "module", "declare"]
    }
}
