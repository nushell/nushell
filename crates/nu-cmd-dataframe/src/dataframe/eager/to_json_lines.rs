use std::{fs::File, io::BufWriter, path::PathBuf};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use polars::prelude::{JsonWriter, SerWriter};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToJsonLines;

impl Command for ToJsonLines {
    fn name(&self) -> &str {
        "dfr to-jsonl"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to a JSON lines file."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to JSON lines file",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-jsonl test.jsonl",
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

    let mut df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let file = File::create(&file_name.item).map_err(|e| {
        ShellError::GenericError(
            "Error with file name".into(),
            e.to_string(),
            Some(file_name.span),
            None,
            Vec::new(),
        )
    })?;
    let buf_writer = BufWriter::new(file);

    JsonWriter::new(buf_writer)
        .finish(df.as_mut())
        .map_err(|e| {
            ShellError::GenericError(
                "Error saving file".into(),
                e.to_string(),
                Some(file_name.span),
                None,
                Vec::new(),
            )
        })?;

    let file_value = Value::string(format!("saved {:?}", &file_name.item), file_name.span);

    Ok(PipelineData::Value(
        Value::list(vec![file_value], call.head),
        None,
    ))
}
