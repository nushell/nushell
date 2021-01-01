use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Which;

#[async_trait]
impl WholeStreamCommand for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .required("application", SyntaxShape::String, "application")
            .switch("all", "list all executables", Some('a'))
    }

    fn usage(&self) -> &str {
        "Finds a program file, alias or custom command."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        which(args).await
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
        UntaggedValue::boolean(builtin).into_value(tag.clone()),
    );

    UntaggedValue::row(map).into_value(tag)
}

macro_rules! create_entry {
    ($arg:expr, $path:expr, $tag:expr, $is_builtin:expr) => {
        entry(
            $arg.clone(),
            UntaggedValue::Primitive(Primitive::String($path.to_string())).into_value($tag.clone()),
            $is_builtin,
            $tag,
        )
    };
}

#[allow(unused)]
macro_rules! entry_path {
    ($arg:expr, $path:expr, $tag:expr) => {
        entry(
            $arg.clone(),
            UntaggedValue::Primitive(Primitive::Path($path)).into_value($tag.clone()),
            false,
            $tag,
        )
    };
}

#[derive(Deserialize, Debug)]
struct WhichArgs {
    application: Tagged<String>,
    all: bool,
}

async fn which(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut output = vec![];
    let scope = args.scope.clone();

    let (WhichArgs { application, all }, _) = args.process().await?;
    let external = application.starts_with('^');
    let item = if external {
        application.item[1..].to_string()
    } else {
        application.item.clone()
    };
    if !external {
        if let Some(entry) = entry_for(&scope, &item, application.tag.clone()) {
            output.push(ReturnSuccess::value(entry));
        }
    }

    #[cfg(feature = "ichwh")]
    {
        if let Ok(paths) = ichwh::which_all(&item).await {
            for path in paths {
                output.push(ReturnSuccess::value(entry_path!(
                    item,
                    path.into(),
                    application.tag.clone()
                )));
            }
        }
    }

    if all {
        Ok(futures::stream::iter(output.into_iter()).to_output_stream())
    } else {
        Ok(futures::stream::iter(output.into_iter().take(1)).to_output_stream())
    }
}

fn entry_for(scope: &Scope, name: &str, tag: Tag) -> Option<Value> {
    if scope.has_custom_command(name) {
        Some(create_entry!(name, "Nushell custom command", tag, false))
    } else if scope.has_command(name) {
        Some(create_entry!(name, "Nushell built-in command", tag, true))
    } else if scope.has_alias(name) {
        Some(create_entry!(name, "Nushell alias", tag, false))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Which;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(Which {})?)
    }
}
