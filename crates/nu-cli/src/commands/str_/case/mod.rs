pub mod camel_case;
pub mod kebab_case;
pub mod pascal_case;
pub mod screaming_snake_case;
pub mod snake_case;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, UntaggedValue, Value};
use nu_source::Tag;
use nu_value_ext::ValueExt;

pub use camel_case::SubCommand as CamelCase;
pub use pascal_case::SubCommand as PascalCase;
pub use screaming_snake_case::SubCommand as ScreamingSnakeCase;
pub use snake_case::SubCommand as SnakeCase;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
}

pub async fn operate<F>(
    args: CommandArgs,

    case_operation: &'static F,
) -> Result<OutputStream, ShellError>
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    let (Arguments { rest }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;
    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag(), &case_operation)?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag(), &case_operation)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

pub fn action<F>(
    input: &Value,
    tag: impl Into<Tag>,
    case_operation: &F,
) -> Result<Value, ShellError>
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::string(case_operation(s)).into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}
