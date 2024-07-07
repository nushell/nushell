mod capitalize;
mod downcase;
mod str_;
mod upcase;

pub use capitalize::SubCommand as StrCapitalize;
pub use downcase::SubCommand as StrDowncase;
pub use str_::Str;
pub use upcase::SubCommand as StrUpcase;

use nu_cmd_base::input_handler::{operate as general_operate, CmdArgument};
use nu_engine::command_prelude::*;

struct Arguments<F: Fn(&str) -> String + Send + Sync + 'static> {
    case_operation: &'static F,
    cell_paths: Option<Vec<CellPath>>,
}

impl<F: Fn(&str) -> String + Send + Sync + 'static> CmdArgument for Arguments<F> {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

pub fn operate<F>(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    case_operation: &'static F,
) -> Result<PipelineData, ShellError>
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
    let args = Arguments {
        case_operation,
        cell_paths,
    };
    general_operate(action, args, input, call.head, engine_state.signals())
}

fn action<F>(input: &Value, args: &Arguments<F>, head: Span) -> Value
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    let case_operation = args.case_operation;
    match input {
        Value::String { val, .. } => Value::string(case_operation(val), head),
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
