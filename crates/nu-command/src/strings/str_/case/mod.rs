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

use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

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
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, case_operation, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, case_operation, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

pub fn action<F>(input: &Value, case_operation: &F, head: Span) -> Value
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
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
