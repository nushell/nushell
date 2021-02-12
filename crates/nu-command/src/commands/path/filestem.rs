use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathFilestem;

#[derive(Deserialize)]
struct PathFilestemArguments {
    prefix: Option<Tagged<String>>,
    suffix: Option<Tagged<String>>,
    replace: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathFilestem {
    fn name(&self) -> &str {
        "path filestem"
    }

    fn signature(&self) -> Signature {
        Signature::build("path filestem")
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with filestem replaced by this string",
                Some('r'),
            )
            .named(
                "prefix",
                SyntaxShape::String,
                "Strip this string from from the beginning of a file name",
                Some('p'),
            )
            .named(
                "suffix",
                SyntaxShape::String,
                "Strip this string from from the end of a file name",
                Some('s'),
            )
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Gets the file stem of a path"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (
            PathFilestemArguments {
                replace,
                prefix,
                suffix,
                rest,
            },
            input,
        ) = args.process().await?;
        let args = Arc::new(DefaultArguments {
            replace: replace.map(|v| v.item),
            prefix: prefix.map(|v| v.item),
            suffix: suffix.map(|v| v.item),
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get filestem of a path",
                example: "echo 'C:\\Users\\joe\\bacon_lettuce.egg' | path filestem",
                result: Some(vec![Value::from("bacon_lettuce")]),
            },
            Example {
                description: "Get filestem of a path, stripped of prefix and suffix",
                example: "echo 'C:\\Users\\joe\\bacon_lettuce.egg.gz' | path filestem -p bacon_ -s .egg.gz",
                result: Some(vec![Value::from("lettuce")]),
            },
            Example {
                description: "Replace the filestem that would be returned",
                example: "echo 'C:\\Users\\joe\\bacon_lettuce.egg.gz' | path filestem -p bacon_ -s .egg.gz -r spam",
                result: Some(vec![Value::from(UntaggedValue::filepath("C:\\Users\\joe\\bacon_spam.egg.gz"))]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get filestem of a path",
                example: "echo '/home/joe/bacon_lettuce.egg' | path filestem",
                result: Some(vec![Value::from("bacon_lettuce")]),
            },
            Example {
                description: "Get filestem of a path, stripped of prefix and suffix",
                example: "echo '/home/joe/bacon_lettuce.egg.gz' | path filestem -p bacon_ -s .egg.gz",
                result: Some(vec![Value::from("lettuce")]),
            },
            Example {
                description: "Replace the filestem that would be returned",
                example: "echo '/home/joe/bacon_lettuce.egg.gz' | path filestem -p bacon_ -s .egg.gz -r spam",
                result: Some(vec![Value::from(UntaggedValue::filepath("/home/joe/bacon_spam.egg.gz"))]),
            },
        ]
    }
}

fn action(path: &Path, args: Arc<DefaultArguments>) -> UntaggedValue {
    let basename = match path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => "".to_string(),
    };

    let suffix = match args.suffix {
        Some(ref suf) => match basename.rmatch_indices(suf).next() {
            Some((i, _)) => basename.split_at(i).1.to_string(),
            None => "".to_string(),
        },
        None => match path.extension() {
            // Prepend '.' since the extension returned comes without it
            Some(ext) => ".".to_string() + &ext.to_string_lossy().to_string(),
            None => "".to_string(),
        },
    };

    let prefix = match args.prefix {
        Some(ref pre) => match basename.matches(pre).next() {
            Some(m) => basename.split_at(m.len()).0.to_string(),
            None => "".to_string(),
        },
        None => "".to_string(),
    };

    let basename_without_prefix = match basename.matches(&prefix).next() {
        Some(m) => basename.split_at(m.len()).1.to_string(),
        None => basename,
    };

    let stem = match basename_without_prefix.rmatch_indices(&suffix).next() {
        Some((i, _)) => basename_without_prefix.split_at(i).0.to_string(),
        None => basename_without_prefix,
    };

    match args.replace {
        Some(ref replace) => {
            let new_name = prefix + replace + &suffix;
            UntaggedValue::filepath(path.with_file_name(&new_name))
        }
        None => UntaggedValue::string(stem),
    }
}

#[cfg(test)]
mod tests {
    use super::PathFilestem;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathFilestem {})
    }
}
