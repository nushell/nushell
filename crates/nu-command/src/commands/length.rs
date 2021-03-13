use crate::prelude::*;
use futures::stream::StreamExt;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct Length;

#[derive(Deserialize)]
pub struct LengthArgs {
    column: bool,
}

#[async_trait]
impl WholeStreamCommand for Length {
    fn name(&self) -> &str {
        "length"
    }

    fn signature(&self) -> Signature {
        Signature::build("length").switch(
            "column",
            "Calculate number of columns in table",
            Some('c'),
        )
    }

    fn usage(&self) -> &str {
        "Show the total number of rows or items."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (LengthArgs { column }, input) = args.process().await?;
        let rows: Vec<Value> = input.collect().await;

        let length = if column {
            if rows.is_empty() {
                0
            } else {
                match &rows[0].value {
                    UntaggedValue::Row(dictionary) => dictionary.length(),
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Cannot obtain column length",
                            "cannot obtain column length",
                            tag,
                        ));
                    }
                }
            }
        } else {
            rows.len()
        };

        Ok(OutputStream::one(UntaggedValue::int(length).into_value(tag)))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of entries in a list",
                example: "echo [1 2 3 4 5] | length",
                result: Some(vec![UntaggedValue::int(5).into()]),
            },
            Example {
                description: "Count the number of columns in the calendar table",
                example: "cal | length -c",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Length;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Length {})
    }
}
