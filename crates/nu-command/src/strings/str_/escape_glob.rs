use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str escape-glob"
    }

    fn signature(&self) -> Signature {
        Signature::build("str escape-glob")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, turn strings at the given cell paths into substrings.",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Escape glob pattern."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "list", "ls"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError {
                error: "str escape-glob is deprecated".into(),
                msg: "if you are trying to escape a variable, you don't need to do it now".into(),
                span: Some(call.head),
                help: Some("Remove `str escape-glob` call".into()),
                inner: vec![],
            },
        );
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments { cell_paths };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "escape glob pattern before list",
            example: r#"let f = 'test[a]'; ls ($f | str escape-glob)"#,
            result: None,
        }]
    }
}

fn action(input: &Value, _args: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val: s, .. } => Value::string(nu_glob::Pattern::escape(s), input.span()),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}
