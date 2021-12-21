use std::{fs::File, path::PathBuf};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};
use polars::prelude::ParquetWriter;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToParquet;

impl Command for ToParquet {
    fn name(&self) -> &str {
        "dfr to-parquet"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to parquet file"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to csv file",
            example: "[[a b]; [1 2] [3 4]] | dfr to-df | dfr to-parquet test.parquet",
            result: None,
        }]
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
    let file_name: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let file = File::create(&file_name.item).map_err(|e| {
        ShellError::SpannedLabeledError(
            "Error with file name".into(),
            e.to_string(),
            file_name.span,
        )
    })?;

    ParquetWriter::new(file).finish(df.as_ref()).map_err(|e| {
        ShellError::SpannedLabeledError("Error saving file".into(), e.to_string(), file_name.span)
    })?;

    let file_value = Value::String {
        val: format!("saved {:?}", &file_name.item),
        span: file_name.span,
    };

    Ok(PipelineData::Value(
        Value::List {
            vals: vec![file_value],
            span: call.head,
        },
        None,
    ))
}
