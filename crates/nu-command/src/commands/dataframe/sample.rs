use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape};

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe sample"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Create sample dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe sample")
            .named(
                "n_rows",
                SyntaxShape::Number,
                "number of rows to be taken from dataframe",
                Some('n'),
            )
            .named(
                "fraction",
                SyntaxShape::Number,
                "fraction of dataframe to be taken",
                Some('f'),
            )
            .switch("replace", "sample with replace", Some('e'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sample rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe sample -r 1",
                result: None,
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example:
                    "[[a b]; [1 2] [3 4] [5 6]] | dataframe to-df | dataframe sample -f 0.5 -e",
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let rows: Option<Tagged<usize>> = args.get_flag("n_rows")?;
    let fraction: Option<Tagged<f64>> = args.get_flag("fraction")?;
    let replace: bool = args.has_flag("replace");

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = match (rows, fraction) {
        (Some(rows), None) => df
            .as_ref()
            .sample_n(rows.item, replace)
            .map_err(|e| parse_polars_error::<&str>(&e, &rows.tag.span, None)),
        (None, Some(frac)) => df
            .as_ref()
            .sample_frac(frac.item, replace)
            .map_err(|e| parse_polars_error::<&str>(&e, &frac.tag.span, None)),
        (Some(_), Some(_)) => Err(ShellError::labeled_error(
            "Incompatible flags",
            "Only one selection criterion allowed",
            &tag,
        )),
        (None, None) => Err(ShellError::labeled_error_with_secondary(
            "No selection",
            "No selection criterion was found",
            &tag,
            "Perhaps you want to use the flag -n or -f",
            &tag,
        )),
    }?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
