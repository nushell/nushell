use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe show"
    }

    fn usage(&self) -> &str {
        "Show dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe load")
            .named(
                "rows",
                SyntaxShape::Number,
                "number of rows to be shown",
                Some('r'),
            )
            .switch("tail", "shows tail rows", Some('t'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        show(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shows head rows from dataframe",
                example: "echo [[a b]; [1 2] [3 4]] | dataframe | dataframe show",
                result: None,
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "echo [[a b]; [1 2] [3 4] [5 6]] | dataframe | dataframe show -t -r 1",
                result: None,
            },
        ]
    }
}

fn show(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let rows: Option<Tagged<usize>> = args.get_flag("rows")?;
    let tail: bool = args.has_flag("tail");

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(df) = value.value {
                let rows = rows.map(|v| v.item);
                let values = if tail { df.tail(rows)? } else { df.head(rows)? };

                Ok(OutputStream::from_stream(values.into_iter()))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}
