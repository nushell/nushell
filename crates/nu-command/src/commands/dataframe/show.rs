use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape};

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe show"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Converts a section of the dataframe to a Table or List value"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe show")
            .named(
                "n_rows",
                SyntaxShape::Number,
                "number of rows to be shown",
                Some('n'),
            )
            .switch("tail", "shows tail rows", Some('t'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shows head rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe show",
                result: None,
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "[[a b]; [1 2] [3 4] [5 6]] | dataframe to-df | dataframe show -t -n 1",
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let rows: Option<Tagged<usize>> = args.get_flag("n_rows")?;
    let tail: bool = args.has_flag("tail");

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let rows = rows.map(|v| v.item);
    let values = if tail { df.tail(rows)? } else { df.head(rows)? };

    Ok(OutputStream::from_stream(values.into_iter()))
}
