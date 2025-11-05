use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrRepeat;

impl Command for StrRepeat {
    fn name(&self) -> &str {
        "str repeat"
    }

    fn signature(&self) -> Signature {
        Signature::build("str repeat")
            .input_output_types(
                vec![
                    (Type::String, Type::String),
                    (
                        Type::List(Box::new(Type::String)),
                        Type::List(Box::new(Type::String)),
                    ),
                    (Type::Table(vec![].into()), Type::Table(vec![].into())),
                ]
                .into(),
            )
            .required(
                "count",
                SyntaxShape::Int,
                "number of times to repeat the string",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, repeat strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Repeat a string n times."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["replicate", "duplicate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let count: usize = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;

        let args = RepeatArguments { count, cell_paths };

        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Repeat a string 3 times",
                example: "'na' | str repeat 3",
                result: Some(Value::test_string("nanana")),
            },
            Example {
                description: "Repeat a string in a specific column",
                example: "[[sound]; ['kwa']] | str repeat 2 sound",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "sound" => Value::test_string("kwakwa"),
                })])),
            },
        ]
    }
}

struct RepeatArguments {
    count: usize,
    cell_paths: Vec<CellPath>,
}

impl CmdArgument for RepeatArguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        if self.cell_paths.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.cell_paths))
        }
    }
}

fn action(input: &Value, args: &RepeatArguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::string(val.repeat(args.count), head),
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}
