mod command;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{ColumnPath, Primitive, ReturnSuccess, UntaggedValue, Value};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;
use std::iter::FromIterator;

pub use command::SubCommand as Trim;

struct Arguments {
    character: Option<Tagged<char>>,
    column_paths: Vec<ColumnPath>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct ClosureFlags {
    all_flag: bool,
    left_trim: bool,
    right_trim: bool,
    format_flag: bool,
    both_flag: bool,
}

pub fn operate<F>(args: CommandArgs, trim_operation: &'static F) -> Result<ActionStream, ShellError>
where
    F: Fn(&str, Option<char>, &ClosureFlags) -> String + Send + Sync + 'static,
{
    let (options, closure_flags, input) = (
        Arc::new(Arguments {
            character: args.get_flag("char")?,
            column_paths: args.rest(0)?,
        }),
        ClosureFlags {
            all_flag: args.has_flag("all"),
            left_trim: args.has_flag("left"),
            right_trim: args.has_flag("right"),
            format_flag: args.has_flag("format"),
            both_flag: args.has_flag("both")
                || (!args.has_flag("all")
                    && !args.has_flag("left")
                    && !args.has_flag("right")
                    && !args.has_flag("format")), // this is the case if no flags are provided
        },
        args.input,
    );
    let to_trim = options.character.as_ref().map(|tagged| tagged.item);
    Ok(input
        .map(move |v| {
            if options.column_paths.is_empty() {
                ReturnSuccess::value(action(
                    &v,
                    v.tag(),
                    to_trim,
                    &closure_flags,
                    &trim_operation,
                    ActionMode::Global,
                )?)
            } else {
                let mut ret = v;

                for path in &options.column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| {
                            action(
                                old,
                                old.tag(),
                                to_trim,
                                &closure_flags,
                                &trim_operation,
                                ActionMode::Local,
                            )
                        }),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .into_action_stream())
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
    closure_flags: &ClosureFlags,
    trim_operation: &F,
    mode: ActionMode,
) -> Result<Value, ShellError>
where
    F: Fn(&str, Option<char>, &ClosureFlags) -> String + Send + Sync + 'static,
{
    let tag = tag.into();
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::string(trim_operation(s, char_, closure_flags)).into_value(tag))
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
                                action(v, tag.clone(), char_, closure_flags, trim_operation, mode)?,
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
                            action(v, tag.clone(), char_, closure_flags, trim_operation, mode)
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
