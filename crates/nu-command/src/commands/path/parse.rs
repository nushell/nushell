use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
#[cfg(windows)]
use std::path::Component;
use std::path::Path;

pub struct PathParse;

struct PathParseArguments {
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathParseArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathParse {
    fn name(&self) -> &str {
        "path parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("path parse")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Convert a path into structured data"
    }

    fn extra_usage(&self) -> &str {
        "On Windows, extra 'prefix' column is added."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let cmd_args = Arc::new(PathParseArguments {
            rest: args.rest_args()?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Parse a path",
            example: r"echo 'C:\Users\viking\spam.txt | path parse",
            result: None,
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Parse a path",
            example: r"echo '/home/viking/spam.txt' | path parse",
            result: None,
        }]
    }
}

fn action(path: &Path, tag: Tag, _args: &PathParseArguments) -> Value {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let filestem = path
        .file_stem()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();
    let extension = path
        .extension()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();

    let mut dict = TaggedDictBuilder::new(&tag);
    #[cfg(windows)]
    {
        // The prefix is only valid on Windows. On non-Windows, it's always empty.
        let prefix = match path.components().next() {
            Some(Component::Prefix(prefix_component)) => {
                prefix_component.as_os_str().to_string_lossy()
            }
            _ => "".into(),
        };
        dict.insert_untagged("prefix", UntaggedValue::string(prefix));
    }
    dict.insert_untagged("parent", UntaggedValue::filepath(parent));
    dict.insert_untagged("stem", UntaggedValue::string(filestem));
    dict.insert_untagged("extension", UntaggedValue::string(extension));

    dict.into_value()
}

#[cfg(test)]
mod tests {
    use super::PathParse;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathParse {})
    }
}
