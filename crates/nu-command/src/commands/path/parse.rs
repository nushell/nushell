use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use std::path::Path;

pub struct PathParse;

#[derive(Deserialize)]
struct PathParseArguments {
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathParseArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

#[async_trait]
impl WholeStreamCommand for PathParse {
    fn name(&self) -> &str {
        "path parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("path parse")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Convert path into structured data"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathParseArguments { rest }, input) = args.process().await?;
        let args = Arc::new(PathParseArguments { rest });
        operate(input, &action, tag.span, args).await
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

fn action(path: &Path, tag: Tag, _args: &PathParseArguments) -> Result<Value, ShellError> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let filestem = match path.file_stem() {
        Some(stem) => match stem.to_str() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "can not convert file stem to unicode",
                ))
            }
        },
        None => "",
    };
    let extension = match path.extension() {
        Some(stem) => match stem.to_str() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "can not convert extension to unicode",
                ))
            }
        },
        None => "",
    };

    let mut dict = TaggedDictBuilder::new(&tag);
    #[cfg(windows)]
    {
        // The prefix is only valid on Windows. On non-Windows, it's always empty.
        let prefix = match path.components().next() {
            Some(Component::Prefix(prefix_component)) => {
                match prefix_component.as_os_str().to_str() {
                    Some(s) => s,
                    None => {
                        return Err(ShellError::untagged_runtime_error(
                            "can not convert prefix to unicode",
                        ))
                    }
                }
            }
            _ => "",
        };
        dict.insert_untagged("prefix", UntaggedValue::string(prefix));
    }
    dict.insert_untagged("parent", UntaggedValue::filepath(parent));
    dict.insert_untagged("stem", UntaggedValue::string(filestem));
    dict.insert_untagged("extension", UntaggedValue::string(extension));

    Ok(dict.into_value())
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
