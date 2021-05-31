use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls sample"
    }

    fn usage(&self) -> &str {
        "Create sample dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls load")
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
                example: "echo [[a b]; [1 2] [3 4]] | pls load | pls sample -r 1",
                result: None,
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example: "echo [[a b]; [1 2] [3 4] [5 6]] | pls load | pls sample -f 0.5 -e",
                result: None,
            },
        ]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let rows: Option<Tagged<usize>> = args.get_flag("n_rows")?;
    let fraction: Option<Tagged<f64>> = args.get_flag("fraction")?;
    let replace: bool = args.has_flag("replace");

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) = value.value {
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

                let value = Value {
                    value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame::new(
                        res,
                    ))),
                    tag: tag.clone(),
                };

                Ok(OutputStream::one(value))
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
