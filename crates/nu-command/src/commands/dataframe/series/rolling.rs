use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue,
};
use nu_source::Tagged;
use polars::prelude::DataType;

enum RollType {
    Min,
    Max,
    Sum,
    Mean,
}

impl RollType {
    fn from_str(roll_type: &str, span: &Span) -> Result<Self, ShellError> {
        match roll_type {
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            "mean" => Ok(Self::Mean),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Wrong operation",
                "Operation not valid for rolling",
                span,
                "Perhaps you want to use: max, min, sum, mean",
                span,
            )),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            RollType::Min => "rolling_min",
            RollType::Max => "rolling_max",
            RollType::Sum => "rolling_sum",
            RollType::Mean => "rolling_mean",
        }
    }
}

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe rolling"
    }

    fn usage(&self) -> &str {
        "[Series] Rolling calculation for a series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe rolling")
            .required("type", SyntaxShape::String, "rolling operation")
            .required("window", SyntaxShape::Int, "Window size for rolling")
            .switch("ignore_nulls", "Ignore nulls in column", Some('i'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rolling sum for a series",
                example:
                    "[1 2 3 4 5] | dataframe to-df | dataframe rolling sum 2 | dataframe drop-nulls",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0_rolling_sum".to_string(),
                        vec![
                            UntaggedValue::int(3).into(),
                            UntaggedValue::int(5).into(),
                            UntaggedValue::int(7).into(),
                            UntaggedValue::int(9).into(),
                        ],
                    )],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Rolling max for a series",
                example:
                    "[1 2 3 4 5] | dataframe to-df | dataframe rolling max 2 | dataframe drop-nulls",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0_rolling_max".to_string(),
                        vec![
                            UntaggedValue::int(2).into(),
                            UntaggedValue::int(3).into(),
                            UntaggedValue::int(4).into(),
                            UntaggedValue::int(5).into(),
                        ],
                    )],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let roll_type: Tagged<String> = args.req(0)?;
    let window_size: Tagged<i64> = args.req(1)?;
    let ignore_nulls = args.has_flag("ignore_nulls");

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let series = df.as_series(&df_tag.span)?;

    if let DataType::Object(_) = series.dtype() {
        return Err(ShellError::labeled_error(
            "Found object series",
            "Series of type object cannot be used for rolling operation",
            &df_tag.span,
        ));
    }

    let roll_type = RollType::from_str(&roll_type.item, &roll_type.tag.span)?;
    let res = match roll_type {
        RollType::Max => series.rolling_max(
            window_size.item as u32,
            None,
            ignore_nulls,
            window_size.item as u32,
        ),
        RollType::Min => series.rolling_min(
            window_size.item as u32,
            None,
            ignore_nulls,
            window_size.item as u32,
        ),
        RollType::Sum => series.rolling_sum(
            window_size.item as u32,
            None,
            ignore_nulls,
            window_size.item as u32,
        ),
        RollType::Mean => series.rolling_mean(
            window_size.item as u32,
            None,
            ignore_nulls,
            window_size.item as u32,
        ),
    };

    let mut res = res.map_err(|e| parse_polars_error::<&str>(&e, &df_tag.span, None))?;

    let name = format!("{}_{}", series.name(), roll_type.to_str());
    res.rename(&name);

    let df = NuDataFrame::try_from_series(vec![res], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
}

#[cfg(test)]
mod tests {
    use super::DataFrame;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test_dataframe as test_examples;

        test_examples(DataFrame {})
    }
}
