use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathFilestem;

#[derive(Deserialize)]
struct PathFilestemArguments {
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
                "Replace filestem with this string",
                Some('r'),
            )
            .named(
                "suffix",
                SyntaxShape::String,
                "Manually specify filename suffix",
                Some('s'),
            )
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the filestem of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathFilestemArguments { replace, suffix, rest }, input) =
            args.process(&registry).await?;
        let args = Arc::new(DefaultArguments {
            replace: replace.map(|v| v.item),
            suffix: suffix.map(|v| v.item),
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get filestem of a path",
            example: "echo '/home/joe/test.txt' | path filestem",
            result: Some(vec![Value::from("test")]),
        }]
    }
}

fn action(path: &Path, args: Arc<DefaultArguments>) -> UntaggedValue {
    let basename = match path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => "".to_string(),
    };

    // std::str::pattern::Pattern would be better but is unstable

    let suffix = match args.suffix {
        Some(ref suf) => match basename.rmatch_indices(suf).next() {
            Some((i, _)) => basename.split_at(i).1.to_string(),
            None => "".to_string(),
        },
        None => match path.extension() {
            Some(ext) => ".".to_string() + &ext.to_string_lossy().to_string(),
            None => "".to_string(),
        },
    };

    let stem = match basename.rmatch_indices(&suffix).next() {
        Some((i, _)) => basename.split_at(i).0.to_string(),
        None => basename,
    };

    match args.replace {
        Some(ref replace) => {
            let new_name = replace.to_string() + &suffix;
            UntaggedValue::string(
                path.with_file_name(&new_name).to_string_lossy()
            )
        },
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

        Ok(test_examples(PathFilestem {})?)
    }
}
