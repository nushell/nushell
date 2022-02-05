use crate::prelude::*;

use nu_engine::WholeStreamCommand;
use nu_errors::{ProximateShellError, ShellError};
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_value_ext::get_data_by_column_path;

pub struct Compact;

impl WholeStreamCommand for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "the columns to compact from the table",
        )
    }

    fn usage(&self) -> &str {
        "Creates a table with non-empty rows."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        compact(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter out all directory entries having no 'target'",
                example: "ls -la | compact target",
                result: None,
            },
            Example {
                description: "Works with properties containing special characters",
                example: r#"'[{ "labels": { "app.kubernetes.io/name": "my-great-app" } }, { "labels": {} }]' | from json | compact labels.'app.kubernetes.io/name' | select labels.'app.kubernetes.io/name'"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                    "labels_app_kubernetes_io/name".into() => "my-great-app".into(),
                })
                .into()]),
            },
        ]
    }
}

pub fn compact(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let paths: Vec<ColumnPath> = args.rest(0)?;

    Ok(args
        .input
        .filter_map(move |item| {
            for path in &paths {
                match get_data_by_column_path(&item, &path, |_, _, e| e) {
                    Ok(_) => {}
                    Err(ShellError {
                        error: ProximateShellError::MissingProperty { .. },
                        ..
                    }) => return None,
                    Err(e) => return Some(Err(e)),
                };
            }

            Some(ReturnSuccess::value(item))
        })
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Compact;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Compact {})
    }
}
