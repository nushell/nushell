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
        "Finds a program file."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        which(args, registry)
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

    UntaggedValue::row(map).into_value(tag)
}

macro_rules! entry_builtin {
    ($arg:expr, $tag:expr) => {
        entry(
            $arg.clone(),
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

fn which(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let mut all = true;
    let stream = async_stream! {
        let (WhichArgs { application, all: all_items }, _) = args.process(&registry).await?;
        all = all_items;
        let external = application.starts_with('^');
        let item = if external {
            application.item[1..].to_string()
        } else {
            application.item.clone()
        };
        if !external {
            let builtin = registry.has(&item);
            if builtin {
                yield ReturnSuccess::value(entry_builtin!(item, application.tag.clone()));
            }
        }

        if let Ok(paths) = ichwh::which_all(&item).await {
            for path in paths {
                yield ReturnSuccess::value(entry_path!(item, path.into(), application.tag.clone()));
            }
        }
    };

    if all {
        Ok(stream.to_output_stream())
    } else {
        Ok(stream.take(1).to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Which;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Which {})
    }
}
