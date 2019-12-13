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

    let mut rows: VecDeque<Value> = VecDeque::new();

    if let Some(ref positional) = &args.call_info.args.positional {
        for i in positional {
            match i {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    if registry.has(s) {
                        rows.push_back(entry_builtin(s, tag.clone()));
                    } else if let Ok(ok) = which::which(&s) {
                        rows.push_back(entry_path(s, ok, tag.clone()))
                    }
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Expected a filename to find",
                        "needs a filename",
                        tag,
                    ));
                }
            }
        }
    } else {
        return Err(ShellError::labeled_error(
            "Expected a binary to find",
            "needs application name",
            tag,
        ));
    }

    Ok(rows.to_output_stream())
}

/// Shortcuts for creating an entry to the output table
fn entry(arg: impl Into<String>, path: Value, tag: Tag) -> Value {
    let mut map = IndexMap::new();
    map.insert(
        "arg".to_string(),
        UntaggedValue::Primitive(Primitive::String(arg.into())).into_value(tag.clone()),
    );
    map.insert("path".to_string(), path);

    UntaggedValue::row(map).into_value(tag.clone())
}

fn entry_builtin(arg: impl Into<String>, tag: Tag) -> Value {
    entry(
        arg,
        UntaggedValue::Primitive(Primitive::String("nushell built-in command".to_string()))
            .into_value(tag.clone()),
        tag,
    )
}

fn entry_path(arg: impl Into<String>, path: std::path::PathBuf, tag: Tag) -> Value {
    entry(
        arg,
        UntaggedValue::Primitive(Primitive::Path(path)).into_value(tag.clone()),
        tag,
    )
}
