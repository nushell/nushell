use std::{fs::File, path::PathBuf};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use polars_io::avro::{AvroCompression, AvroWriter};
use polars_io::SerWriter;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToAvro;

impl Command for ToAvro {
    fn name(&self) -> &str {
        "dfr to-avro"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to avro file."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "compression",
                SyntaxShape::String,
                "use compression, supports deflate or snappy",
                Some('c'),
            )
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to avro file",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-avro test.avro",
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

fn get_compression(call: &Call) -> Result<Option<AvroCompression>, ShellError> {
    if let Some((compression, span)) = call
        .get_flag_expr("compression")
        .and_then(|e| e.as_string().map(|s| (s, e.span)))
    {
        match compression.as_ref() {
            "snappy" => Ok(Some(AvroCompression::Snappy)),
            "deflate" => Ok(Some(AvroCompression::Deflate)),
            _ => Err(ShellError::IncorrectValue {
                msg: "compression must be one of deflate or snappy".to_string(),
                val_span: span,
                call_span: span,
            }),
        }
    } else {
        Ok(None)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let file_name: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;
    let compression = get_compression(call)?;

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

    AvroWriter::new(file)
        .with_compression(compression)
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
