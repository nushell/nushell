use crate::prelude::*;

use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct Length;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let column = args.has_flag("column");
        let input = args.input;

        Ok(CountIterator {
            column,
            input,
            done: false,
            tag,
        }
        .into_output_stream())
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

struct CountIterator {
    column: bool,
    input: InputStream,
    done: bool,
    tag: Tag,
}

impl Iterator for CountIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        self.done = true;

        let length = if self.column {
            if let Some(first) = self.input.next() {
                match &first.value {
                    UntaggedValue::Row(dictionary) => dictionary.length(),
                    _ => {
                        return Some(Value::error(ShellError::labeled_error(
                            "Cannot obtain column length",
                            "cannot obtain column length",
                            self.tag.clone(),
                        )));
                    }
                }
            } else {
                0
            }
        } else {
            let input = &mut self.input;
            input.count()
        };

        Some(UntaggedValue::int(length as i64).into_value(self.tag.clone()))
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
