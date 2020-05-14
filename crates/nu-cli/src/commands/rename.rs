use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Rename;

#[derive(Deserialize)]
pub struct Arguments {
    column_name: Tagged<String>,
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Rename {
    fn name(&self) -> &str {
        "rename"
    }

    fn signature(&self) -> Signature {
        Signature::build("rename")
            .required(
                "column_name",
                SyntaxShape::String,
                "the new name for the first column",
            )
            .rest(SyntaxShape::String, "the new name for additional columns")
    }

    fn usage(&self) -> &str {
        "Creates a new table with columns renamed."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        rename(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Rename a column",
                example: "ls | rename my_name",
            },
            Example {
                description: "Rename many columns",
                example: "echo \"{a: 1, b: 2, c: 3}\" | from json | rename spam eggs cars",
            },
        ]
    }
}

pub fn rename(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (Arguments { column_name, rest }, mut input) = args.process(&registry).await?;
        let mut new_column_names = vec![vec![column_name]];
        new_column_names.push(rest);

        let new_column_names = new_column_names.into_iter().flatten().collect::<Vec<_>>();

        for item in input.next().await {
            let mut result = VecDeque::new();

            if let Value {
                value: UntaggedValue::Row(row),
                tag,
            } = item
            {
                let mut renamed_row = IndexMap::new();

                for (idx, (key, value)) in row.entries.iter().enumerate() {
                    let key = if idx < new_column_names.len() {
                        &new_column_names[idx].item
                    } else {
                        key
                    };

                    renamed_row.insert(key.clone(), value.clone());
                }

                let out = UntaggedValue::Row(renamed_row.into()).into_value(tag);

                yield ReturnSuccess::value(out);
            } else {
                yield ReturnSuccess::value(
                    UntaggedValue::Error(ShellError::labeled_error(
                        "no column names available",
                        "can't rename",
                        &name,
                    ))
                    .into_untagged_value(),
                );
            }
        }
    };

    Ok(stream.to_output_stream())
}
