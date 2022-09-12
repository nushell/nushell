use std::{fs::File, path::PathBuf};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use polars::prelude::{IpcWriter, SerWriter};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToArrow;

impl Command for ToArrow {
    fn name(&self) -> &str {
        "to arrow"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to arrow file"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to arrow file",
            example: "[[a b]; [1 2] [3 4]] | into df | to arrow test.arrow",
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

    let mut file = File::create(&file_name.item).map_err(|e| {
        ShellError::GenericError(
            "Error with file name".into(),
            e.to_string(),
            Some(file_name.span),
            None,
            Vec::new(),
        )
    })?;

    IpcWriter::new(&mut file).finish(df.as_mut()).map_err(|e| {
        ShellError::GenericError(
            "Error saving file".into(),
            e.to_string(),
            Some(file_name.span),
            None,
            Vec::new(),
        )
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
