use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};


fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    if column_paths.is_empty() {
        input.map(move |v| action(&v, head), engine_state.ctrlc.clone())
    } else {
        input.map(
            move |mut v| {
                for path in &column_paths {
                    let r =
                        v.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                v
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::Binary { val, .. } => Value::Int {
            val: val.len() as i64,
            span: head,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with bytes.",
                    other.get_type()
                ),
                head,
            ),
        },
    }
}