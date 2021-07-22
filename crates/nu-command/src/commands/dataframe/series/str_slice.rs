use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape};
use nu_source::Tagged;
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe str-slice"
    }

    fn usage(&self) -> &str {
        "[Series] Slices the string from the start position until the selected length"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe replace")
            .required_named("start", SyntaxShape::Int, "start of slice", Some('s'))
            .named("length", SyntaxShape::Int, "optional length", Some('l'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates slices from the strings",
            example: "[abcded abc321 abc123] | dataframe to-df | dataframe str-slice -s 1 -l 2",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let start: Tagged<i64> = args.req_named("start")?;

    let length: Option<Tagged<i64>> = args.get_flag("length")?;
    let length = length.map(|v| v.item as u64);

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let series = df.as_series(&df_tag.span)?;
    let chunked = series.utf8().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &df_tag.span,
            Some("The str-slice command can only be used with string columns"),
        )
    })?;

    let mut res = chunked
        .str_slice(start.item, length)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    res.rename(series.name());

    let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
}
