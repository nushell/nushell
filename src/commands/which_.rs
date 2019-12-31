use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Which;

impl WholeStreamCommand for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .switch("all", "list all executables")
            .rest(
                SyntaxShape::String,
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
        args.process(registry, which)?.run()
    }
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

macro_rules! entry_builtin {
    ($arg:expr, $tag:expr) => {
        entry(
            $arg,
            UntaggedValue::Primitive(Primitive::String("nushell built-in command".to_string()))
                .into_value($tag.clone()),
            true,
            $tag,
        )
    };
}

macro_rules! entry_path {
    ($arg:expr, $path:expr, $tag:expr) => {
        entry(
            $arg,
            UntaggedValue::Primitive(Primitive::Path($path)).into_value($tag.clone()),
            false,
            $tag,
        )
    };
}

#[derive(Deserialize, Debug)]
struct WhichArgs {
    bin: Tagged<String>,
    all: bool,
}

fn which(
    WhichArgs { bin, all }: WhichArgs,
    RunnableContext { commands, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if all {
        let stream = async_stream! {
            let builtin = commands.has(&bin.item);
            if builtin {
                yield ReturnSuccess::value(entry_builtin!(&bin.item, bin.tag.clone()));
            }

            if let Ok(paths) = ichwh::which_all(&bin.item).await {
                if !builtin && paths.len() == 0 {
                    yield Err(ShellError::labeled_error(
                        "Binary not found for argument, and argument is not a builtin",
                        "not found",
                        &bin.tag,
                    ));
                } else {
                    for path in paths {
                        yield ReturnSuccess::value(entry_path!(&bin.item, path.into(), bin.tag.clone()));
                    }
                }
            } else {
                yield Err(ShellError::labeled_error(
                    "Error trying to find binary for argument",
                    "error",
                    &bin.tag,
                ));
            }
        };

        Ok(stream.to_output_stream())
    } else {
        let stream = async_stream! {
            if commands.has(&bin.item) {
                yield ReturnSuccess::value(entry_builtin!(&bin.item, bin.tag.clone()));
            } else if let Ok(path) = ichwh::which(&bin.item).await {
                yield ReturnSuccess::value(entry_path!(&bin.item, path.into(), bin.tag.clone()));
            } else {
                yield Err(ShellError::labeled_error(
                    "Binary not found for argument, and argument is not a builtin",
                    "not found",
                    &bin.tag,
                ));
            }
        };

        Ok(stream.to_output_stream())
    }
}
