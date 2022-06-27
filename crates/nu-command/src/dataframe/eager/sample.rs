use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct SampleDF;

impl Command for SampleDF {
    fn name(&self) -> &str {
        "sample"
    }

    fn usage(&self) -> &str {
        "Create sample dataframe"
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
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sample rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | into df | sample -n 1",
                result: None, // No expected value because sampling is random
            },
            Example {
                description: "Shows sample row using fraction and replace",
                example: "[[a b]; [1 2] [3 4] [5 6]] | into df | sample -f 0.5 -e",
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
    let rows: Option<Spanned<usize>> = call.get_flag(engine_state, stack, "n-rows")?;
    let fraction: Option<Spanned<f64>> = call.get_flag(engine_state, stack, "fraction")?;
    let seed: Option<u64> = call
        .get_flag::<i64>(engine_state, stack, "seed")?
        .map(|val| val as u64);
    let replace: bool = call.has_flag("replace");
    let shuffle: bool = call.has_flag("shuffle");

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    match (rows, fraction) {
        (Some(rows), None) => df
            .as_ref()
            .sample_n(rows.item, replace, shuffle, seed)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error creating sample".into(),
                    e.to_string(),
                    Some(rows.span),
                    None,
                    Vec::new(),
                )
            }),
        (None, Some(frac)) => df
            .as_ref()
            .sample_frac(frac.item, replace, shuffle, seed)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error creating sample".into(),
                    e.to_string(),
                    Some(frac.span),
                    None,
                    Vec::new(),
                )
            }),
        (Some(_), Some(_)) => Err(ShellError::GenericError(
            "Incompatible flags".into(),
            "Only one selection criterion allowed".into(),
            Some(call.head),
            None,
            Vec::new(),
        )),
        (None, None) => Err(ShellError::GenericError(
            "No selection".into(),
            "No selection criterion was found".into(),
            Some(call.head),
            Some("Perhaps you want to use the flag -n or -f".into()),
            Vec::new(),
        )),
    }
    .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}
