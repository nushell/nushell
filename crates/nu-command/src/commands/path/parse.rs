use super::{column_paths_from_args, operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;
#[cfg(windows)]
use std::path::Component;
use std::path::Path;

pub struct PathParse;

struct PathParseArguments {
    columns: Vec<ColumnPath>,
    extension: Option<Tagged<String>>,
}

impl PathSubcommandArguments for PathParseArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathParse {
    fn name(&self) -> &str {
        "path parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("path parse")
            .named(
                "columns",
                SyntaxShape::Table,
                "Optionally operate by column path",
                Some('c'),
            )
            .named(
                "extension",
                SyntaxShape::String,
                "Manually supply the extension (without the dot)",
                Some('e'),
            )
    }

    fn usage(&self) -> &str {
        "Convert a path into structured data."
    }

    fn extra_usage(&self) -> &str {
        r#"Each path is split into a table with 'parent', 'stem' and 'extension' fields.
On Windows, an extra 'prefix' column is added."#
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathParseArguments {
            columns: column_paths_from_args(&args)?,
            extension: args.get_flag("extension")?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a single path",
                example: r"'C:\Users\viking\spam.txt' | path parse",
                result: None,
            },
            Example {
                description: "Replace a complex extension",
                example: r"'C:\Users\viking\spam.tar.gz' | path parse -e tar.gz | update extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'C:\Users\viking.d' | path parse -e ''",
                result: None,
            },
            Example {
                description: "Parse all paths under the 'name' column",
                example: r"ls | path parse -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a path",
                example: r"'/home/viking/spam.txt' | path parse",
                result: None,
            },
            Example {
                description: "Replace a complex extension",
                example: r"'/home/viking/spam.tar.gz' | path parse -e tar.gz | update extension { 'txt' }",
                result: None,
            },
            Example {
                description: "Ignore the extension",
                example: r"'/etc/conf.d' | path parse -e ''",
                result: None,
            },
            Example {
                description: "Parse all paths under the 'name' column",
                example: r"ls | path parse -c [ name ]",
                result: None,
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathParseArguments) -> Value {
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

    let parent = path.parent().unwrap_or_else(|| "".as_ref());
    dict.insert_untagged("parent", UntaggedValue::filepath(parent));

    let basename = path
        .file_name()
        .unwrap_or_else(|| "".as_ref())
        .to_string_lossy();

    match &args.extension {
        Some(Tagged { item: ext, .. }) => {
            let ext_with_dot = [".", ext].concat();
            if basename.ends_with(&ext_with_dot) && !ext.is_empty() {
                let stem = basename.trim_end_matches(&ext_with_dot);
                dict.insert_untagged("stem", UntaggedValue::string(stem));
                dict.insert_untagged("extension", UntaggedValue::string(ext));
            } else {
                dict.insert_untagged("stem", UntaggedValue::string(basename));
                dict.insert_untagged("extension", UntaggedValue::string(""));
            }
        }
        None => {
            let stem = path
                .file_stem()
                .unwrap_or_else(|| "".as_ref())
                .to_string_lossy();
            let extension = path
                .extension()
                .unwrap_or_else(|| "".as_ref())
                .to_string_lossy();

            dict.insert_untagged("stem", UntaggedValue::string(stem));
            dict.insert_untagged("extension", UntaggedValue::string(extension));
        }
    }

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
