use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature, SyntaxShape};
use nu_source::Tagged;
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe replace"
    }

    fn usage(&self) -> &str {
        "[Series] Replace the leftmost (sub)string by a regex pattern"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe count-null")
            .required_named(
                "pattern",
                SyntaxShape::String,
                "Regex pattern to be matched",
                Some('p'),
            )
            .required_named(
                "replace",
                SyntaxShape::String,
                "replacing string",
                Some('r'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Replaces string",
            example: "[abc abc abc] | dataframe to-series | dataframe replace -p ab -r AB",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let pattern: Tagged<String> = args.req_named("pattern")?;
    let replace: Tagged<String> = args.req_named("replace")?;

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let chunked = series.as_ref().utf8().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &tag.span,
            Some("The replace command can only be used with string columns"),
        )
    })?;

    let res = chunked
        .as_ref()
        .replace(pattern.as_str(), replace.as_str())
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuSeries::series_to_value(
        res.into_series(),
        tag,
    )))
}
