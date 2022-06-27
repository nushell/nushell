use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};
use polars::prelude::{DataType, Duration, IntoSeries, RollingOptionsImpl, SeriesOpsTime};

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
            _ => Err(ShellError::GenericError(
                "Wrong operation".into(),
                "Operation not valid for cumulative".into(),
                Some(span),
                Some("Allowed values: min, max, sum, mean".into()),
                Vec::new(),
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
        "rolling"
    }

    fn usage(&self) -> &str {
        "Rolling calculation for a series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("type", SyntaxShape::String, "rolling operation")
            .required("window", SyntaxShape::Int, "Window size for rolling")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rolling sum for a series",
                example: "[1 2 3 4 5] | into df | rolling sum 2 | drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0_rolling_sum".to_string(),
                        vec![
                            Value::test_int(3),
                            Value::test_int(5),
                            Value::test_int(7),
                            Value::test_int(9),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Rolling max for a series",
                example: "[1 2 3 4 5] | into df | rolling max 2 | drop-nulls",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0_rolling_max".to_string(),
                        vec![
                            Value::test_int(2),
                            Value::test_int(3),
                            Value::test_int(4),
                            Value::test_int(5),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
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
    let window_size: i64 = call.req(engine_state, stack, 1)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?;

    if let DataType::Object(_) = series.dtype() {
        return Err(ShellError::GenericError(
            "Found object series".into(),
            "Series of type object cannot be used for rolling operation".into(),
            Some(call.head),
            None,
            Vec::new(),
        ));
    }

    let roll_type = RollType::from_str(&roll_type.item, roll_type.span)?;

    let rolling_opts = RollingOptionsImpl {
        window_size: Duration::new(window_size),
        min_periods: window_size as usize,
        weights: None,
        center: false,
        by: None,
        closed_window: None,
        tu: None,
    };
    let res = match roll_type {
        RollType::Max => series.rolling_max(rolling_opts),
        RollType::Min => series.rolling_min(rolling_opts),
        RollType::Sum => series.rolling_sum(rolling_opts),
        RollType::Mean => series.rolling_mean(rolling_opts),
    };

    let mut res = res.map_err(|e| {
        ShellError::GenericError(
            "Error calculating rolling values".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;

    let name = format!("{}_{}", series.name(), roll_type.to_str());
    res.rename(&name);

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::eager::DropNulls;
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Rolling {}), Box::new(DropNulls {})])
    }
}
