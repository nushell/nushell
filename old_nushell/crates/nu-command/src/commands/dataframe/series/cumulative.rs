use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue,
};
use nu_source::Tagged;
use polars::prelude::DataType;

enum CumType {
    Min,
    Max,
    Sum,
}

impl CumType {
    fn from_str(roll_type: &str, span: &Span) -> Result<Self, ShellError> {
        match roll_type {
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Wrong operation",
                "Operation not valid for cumulative",
                span,
                "Perhaps you want to use: max, min, sum",
                span,
            )),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            CumType::Min => "cum_min",
            CumType::Max => "cum_max",
            CumType::Sum => "cum_sum",
        }
    }
}

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe cum"
    }

    fn usage(&self) -> &str {
        "[Series] Cumulative calculation for a series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe cum")
            .required("type", SyntaxShape::String, "rolling operation")
            .switch("reverse", "Reverse cumulative calculation", Some('r'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Cumulative sum for a series",
            example: "[1 2 3 4 5] | dataframe to-df | dataframe cum sum",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "0_cum_sum".to_string(),
                    vec![
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(3).into(),
                        UntaggedValue::int(6).into(),
                        UntaggedValue::int(10).into(),
                        UntaggedValue::int(15).into(),
                    ],
                )],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let cum_type: Tagged<String> = args.req(0)?;
    let reverse = args.has_flag("reverse");

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let series = df.as_series(&df_tag.span)?;

    if let DataType::Object(_) = series.dtype() {
        return Err(ShellError::labeled_error(
            "Found object series",
            "Series of type object cannot be used for cumulative operation",
            &df_tag.span,
        ));
    }

    let cum_type = CumType::from_str(&cum_type.item, &cum_type.tag.span)?;
    let mut res = match cum_type {
        CumType::Max => series.cummax(reverse),
        CumType::Min => series.cummin(reverse),
        CumType::Sum => series.cumsum(reverse),
    };

    let name = format!("{}_{}", series.name(), cum_type.to_str());
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
