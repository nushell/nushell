use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::{ProximateShellError, ShellError};
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::{get_data_by_column_path, ValueExt};

pub struct Default;

impl WholeStreamCommand for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            .required(
                "column name",
                SyntaxShape::ColumnPath,
                "the name of the column",
            )
            .required(
                "column value",
                SyntaxShape::Any,
                "the value of the column to default",
            )
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        default(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Give a default 'target' to all file entries",
                example: "ls -la | default target 'nothing'",
                result: None,
            },
            Example {
                description: "Works with properties containing special characters",
                example: r#"'[{ "labels": { "app.kubernetes.io/name": "my-great-app" } }, { "labels": {} }]' | from json | default labels.'app.kubernetes.io/name' unknown | select labels.'app.kubernetes.io/name'"#,
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "labels_app_kubernetes_io/name".into() => "my-great-app".into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "labels_app_kubernetes_io/name".into() => "unknown".into(),
                    })
                    .into(),
                ]),
            },
        ]
    }
}

fn default(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let path: ColumnPath = args.req(0)?;
    let default_value: Value = args.req(1)?;

    let input = args.input;

    Ok(input
        .map(move |item| {
            let should_add = match get_data_by_column_path(&item, &path, |_, _, e| e) {
                Ok(_) => false,
                Err(ShellError {
                    error: ProximateShellError::MissingProperty { .. },
                    ..
                }) => true,
                Err(e) => return Err(e),
            };

            if should_add {
                ReturnSuccess::value(item.insert_data_at_column_path(&path, default_value.clone())?)
            } else {
                ReturnSuccess::value(item)
            }
        })
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Default;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Default {})
    }
}
