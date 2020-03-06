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
                "the name of the column to rename for",
            )
            .rest(
                SyntaxShape::Member,
                "Additional column name(s) to rename for",
            )
    }

    fn usage(&self) -> &str {
        "Creates a new table with columns renamed."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, rename)?.run()
    }
}

pub fn rename(
    Arguments { column_name, rest }: Arguments,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut new_column_names = vec![vec![column_name]];
    new_column_names.push(rest);

    let new_column_names = new_column_names.into_iter().flatten().collect::<Vec<_>>();

    let stream = input
        .values
        .map(move |item| {
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

                result.push_back(ReturnSuccess::value(out));
            } else {
                result.push_back(ReturnSuccess::value(
                    UntaggedValue::Error(ShellError::labeled_error(
                        "no column names available",
                        "can't rename",
                        &name,
                    ))
                    .into_untagged_value(),
                ));
            }

            futures::stream::iter(result)
        })
        .flatten();

    Ok(stream.to_output_stream())
}
