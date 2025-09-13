use crossterm::terminal::size;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TermSize;

impl Command for TermSize {
    fn name(&self) -> &str {
        "term size"
    }

    fn description(&self) -> &str {
        "Returns a record containing the number of columns (width) and rows (height) of the terminal."
    }

    fn signature(&self) -> Signature {
        Signature::build("term size")
            .category(Category::Platform)
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record([("columns".into(), Type::Int), ("rows".into(), Type::Int)].into()),
            )])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return the columns (width) and rows (height) of the terminal",
                example: "term size",
                result: None,
            },
            Example {
                description: "Return the columns (width) of the terminal",
                example: "(term size).columns",
                result: None,
            },
            Example {
                description: "Return the rows (height) of the terminal",
                example: "(term size).rows",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let (cols, rows) = size().unwrap_or((0, 0));

        Ok(Value::record(
            record! {
                "columns" => Value::int(cols as i64, head),
                "rows" => Value::int(rows as i64, head),
            },
            head,
        )
        .into_pipeline_data())
    }
}
