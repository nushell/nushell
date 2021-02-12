mod trim_both_ends;
mod trim_left;
mod trim_right;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, UntaggedValue, Value};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;
use std::iter::FromIterator;

pub use trim_both_ends::SubCommand as Trim;
pub use trim_left::SubCommand as TrimLeft;
pub use trim_right::SubCommand as TrimRight;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
    #[serde(rename(deserialize = "char"))]
    char_: Option<Tagged<char>>,
}

pub async fn operate<F>(
    args: CommandArgs,

    trim_operation: &'static F,
) -> Result<OutputStream, ShellError>
where
    F: Fn(&str, Option<char>) -> String + Send + Sync + 'static,
{
    let (Arguments { rest, char_ }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;
    let to_trim = char_.map(|tagged| tagged.item);

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(
                    &v,
                    v.tag(),
                    to_trim,
                    &trim_operation,
                    ActionMode::Global,
                )?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| {
                            action(old, old.tag(), to_trim, &trim_operation, ActionMode::Local)
                        }),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

#[derive(Debug, Copy, Clone)]
pub enum ActionMode {
    Local,
    Global,
}

pub fn action<F>(
    input: &Value,
    tag: impl Into<Tag>,
    char_: Option<char>,
    trim_operation: &F,
    mode: ActionMode,
) -> Result<Value, ShellError>
where
    F: Fn(&str, Option<char>) -> String + Send + Sync + 'static,
{
    let tag = tag.into();
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::string(trim_operation(s, char_)).into_value(tag))
        }
        other => match mode {
            ActionMode::Global => match other {
                UntaggedValue::Row(dictionary) => {
                    let results: Result<Vec<(String, Value)>, ShellError> = dictionary
                        .entries()
                        .iter()
                        .map(|(k, v)| -> Result<_, ShellError> {
                            Ok((
                                k.clone(),
                                action(&v, tag.clone(), char_, trim_operation, mode)?,
                            ))
                        })
                        .collect();
                    let indexmap = IndexMap::from_iter(results?);
                    Ok(UntaggedValue::Row(indexmap.into()).into_value(tag))
                }
                UntaggedValue::Table(values) => {
                    let values: Result<Vec<Value>, ShellError> = values
                        .iter()
                        .map(|v| -> Result<_, ShellError> {
                            action(v, tag.clone(), char_, trim_operation, mode)
                        })
                        .collect();
                    Ok(UntaggedValue::Table(values?).into_value(tag))
                }
                _ => Ok(input.clone()),
            },
            ActionMode::Local => {
                let got = format!("got {}", other.type_name());
                Err(ShellError::labeled_error(
                    "value is not string",
                    got,
                    tag.span,
                ))
            }
        },
    }
}
