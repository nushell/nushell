use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
};
use polars::prelude::{DataType, IntoSeries, RollingOptions};

enum RollType {
    Min,
    Max,
    Sum,
    Mean,
}

impl RollType {
    fn from_str(roll_type: &str, span: Span) -> Result<Self, ShellError> {
        match roll_type {
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            "mean" => Ok(Self::Mean),
            _ => Err(ShellError::SpannedLabeledErrorHelp(
                "Wrong operation".into(),
                "Operation not valid for cumulative".into(),
                span,
                "Allowed values: min, max, sum, mean".into(),
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

#[derive(Clone)]
pub struct Rolling;

impl Command for Rolling {
    fn name(&self) -> &str {
        "dfr rolling"
    }

    fn usage(&self) -> &str {
        "Rolling calculation for a series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("type", SyntaxShape::String, "rolling operation")
            .required("window", SyntaxShape::Int, "Window size for rolling")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rolling sum for a series",
                example: "[1 2 3 4 5] | dfr to-df | dfr rolling sum 2 | dfr drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0_rolling_sum".to_string(),
                        vec![3.into(), 5.into(), 7.into(), 9.into()],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Rolling max for a series",
                example: "[1 2 3 4 5] | dfr to-df | dfr rolling max 2 | dfr drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0_rolling_max".to_string(),
                        vec![2.into(), 3.into(), 4.into(), 5.into()],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let roll_type: Spanned<String> = call.req(engine_state, stack, 0)?;
    let window_size: usize = call.req(engine_state, stack, 1)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?;

    if let DataType::Object(_) = series.dtype() {
        return Err(ShellError::SpannedLabeledError(
            "Found object series".into(),
            "Series of type object cannot be used for rolling operation".into(),
            call.head,
        ));
    }

    let roll_type = RollType::from_str(&roll_type.item, roll_type.span)?;

    let rolling_opts = RollingOptions {
        window_size,
        min_periods: window_size,
        weights: None,
        center: false,
    };
    let res = match roll_type {
        RollType::Max => series.rolling_max(rolling_opts),
        RollType::Min => series.rolling_min(rolling_opts),
        RollType::Sum => series.rolling_sum(rolling_opts),
        RollType::Mean => series.rolling_mean(rolling_opts),
    };

    let mut res = res.map_err(|e| {
        ShellError::SpannedLabeledError(
            "Error calculating rolling values".into(),
            e.to_string(),
            call.head,
        )
    })?;

    let name = format!("{}_{}", series.name(), roll_type.to_str());
    res.rename(&name);

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::super::DropNulls;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Rolling {}), Box::new(DropNulls {})])
    }
}
