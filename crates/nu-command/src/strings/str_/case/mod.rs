pub mod camel_case;
pub mod capitalize;
pub mod downcase;
pub mod kebab_case;
pub mod pascal_case;
pub mod screaming_snake_case;
pub mod snake_case;
pub mod str_;
pub mod title_case;
pub mod upcase;

pub use camel_case::SubCommand as StrCamelCase;
pub use capitalize::SubCommand as StrCapitalize;
pub use downcase::SubCommand as StrDowncase;
pub use kebab_case::SubCommand as StrKebabCase;
pub use pascal_case::SubCommand as StrPascalCase;
pub use screaming_snake_case::SubCommand as StrScreamingSnakeCase;
pub use snake_case::SubCommand as StrSnakeCase;
pub use str_::Str;
pub use title_case::SubCommand as StrTitleCase;
pub use upcase::SubCommand as StrUpcase;

use nu_engine::CallExt;

use crate::input_handler::{operate as general_operate, CmdArgument};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

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
    general_operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn action<F>(input: &Value, args: &Arguments<F>, head: Span) -> Value
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    let case_operation = args.case_operation;
    match input {
        Value::String { val, .. } => Value::String {
            val: case_operation(val),
            span: head,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                head,
            ),
        },
    }
}
