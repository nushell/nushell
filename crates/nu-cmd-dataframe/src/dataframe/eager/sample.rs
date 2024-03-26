use crate::dataframe::values::NuDataFrame;
use nu_engine::command_prelude::*;

use polars::{prelude::NamedFrom, series::Series};

#[derive(Clone)]
pub struct SampleDF;

impl Command for SampleDF {
    fn name(&self) -> &str {
        "dfr sample"
    }

    fn usage(&self) -> &str {
        "Create sample dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "n-rows",
                SyntaxShape::Int,
                "number of rows to be taken from dataframe",
                Some('n'),
            )
            .named(
                "fraction",
                SyntaxShape::Number,
                "fraction of dataframe to be taken",
                Some('f'),
            )
            .named(
                "seed",
                SyntaxShape::Number,
                "seed for the selection",
                Some('s'),
            )
            .switch("replace", "sample with replace", Some('e'))
            .switch("shuffle", "shuffle sample", Some('u'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sample rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr sample --n-rows 1",
                result: None, // No expected value because sampling is random
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example:
                    "[[a b]; [1 2] [3 4] [5 6]] | dfr into-df | dfr sample --fraction 0.5 --replace",
                result: None, // No expected value because sampling is random
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
    let rows: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "n-rows")?;
    let fraction: Option<Spanned<f64>> = call.get_flag(engine_state, stack, "fraction")?;
    let seed: Option<u64> = call
        .get_flag::<i64>(engine_state, stack, "seed")?
        .map(|val| val as u64);
    let replace: bool = call.has_flag(engine_state, stack, "replace")?;
    let shuffle: bool = call.has_flag(engine_state, stack, "shuffle")?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    match (rows, fraction) {
        (Some(rows), None) => df
            .as_ref()
            .sample_n(&Series::new("s", &[rows.item]), replace, shuffle, seed)
            .map_err(|e| ShellError::GenericError {
                error: "Error creating sample".into(),
                msg: e.to_string(),
                span: Some(rows.span),
                help: None,
                inner: vec![],
            }),
        (None, Some(frac)) => df
            .as_ref()
            .sample_frac(&Series::new("frac", &[frac.item]), replace, shuffle, seed)
            .map_err(|e| ShellError::GenericError {
                error: "Error creating sample".into(),
                msg: e.to_string(),
                span: Some(frac.span),
                help: None,
                inner: vec![],
            }),
        (Some(_), Some(_)) => Err(ShellError::GenericError {
            error: "Incompatible flags".into(),
            msg: "Only one selection criterion allowed".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        }),
        (None, None) => Err(ShellError::GenericError {
            error: "No selection".into(),
            msg: "No selection criterion was found".into(),
            span: Some(call.head),
            help: Some("Perhaps you want to use the flag -n or -f".into()),
            inner: vec![],
        }),
    }
    .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}
