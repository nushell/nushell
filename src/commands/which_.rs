use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Which;

impl WholeStreamCommand for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which").rest(
            SyntaxShape::Any,
            "the names of the commands to find the path to",
        )
    }

    fn usage(&self) -> &str {
        "Finds a program file."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        which(args, registry)
    }
}

pub fn which(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.call_info.name_tag.clone();

    let rows = if let Some(ref positional) = args.call_info.args.positional {
        positional
            .iter()
            .map(|i| match i {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    if registry.has(s) {
                        Ok(entry_builtin(s, tag.clone()))
                    } else if let Ok(ok) = which::which(&s) {
                        Ok(entry_path(s, ok, tag.clone()))
                    } else {
                        Err(ShellError::labeled_error(
                            "Binary not found for argument, and argument is not a builtin",
                            "not found",
                            tag,
                        ))
                    }
                }
                Value { tag, .. } => Err(ShellError::labeled_error(
                    "Expected a filename to find",
                    "needs a filename",
                    tag,
                )),
            })
            .collect::<Result<VecDeque<_>, _>>()
    } else {
        Err(ShellError::labeled_error(
            "Expected a binary to find",
            "needs application name",
            tag,
        ))
    }?;

    Ok(rows.to_output_stream())
}

/// Shortcuts for creating an entry to the output table
fn entry(arg: impl Into<String>, path: Value, builtin: bool, tag: Tag) -> Value {
    let mut map = IndexMap::new();
    map.insert(
        "arg".to_string(),
        UntaggedValue::Primitive(Primitive::String(arg.into())).into_value(tag.clone()),
    );
    map.insert("path".to_string(), path);
    map.insert(
        "builtin".to_string(),
        UntaggedValue::Primitive(Primitive::Boolean(builtin)).into_value(tag.clone()),
    );

    UntaggedValue::row(map).into_value(tag.clone())
}

fn entry_builtin(arg: impl Into<String>, tag: Tag) -> Value {
    entry(
        arg,
        UntaggedValue::Primitive(Primitive::String("nushell built-in command".to_string()))
            .into_value(tag.clone()),
        true,
        tag,
    )
}

fn entry_path(arg: impl Into<String>, path: std::path::PathBuf, tag: Tag) -> Value {
    entry(
        arg,
        UntaggedValue::Primitive(Primitive::Path(path)).into_value(tag.clone()),
        false,
        tag,
    )
}
