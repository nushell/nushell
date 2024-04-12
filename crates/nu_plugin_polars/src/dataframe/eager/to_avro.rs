use std::{fs::File, path::PathBuf};

use nu_path::expand_path_with;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value,
};
use polars_io::avro::{AvroCompression, AvroWriter};
use polars_io::SerWriter;

use crate::PolarsPlugin;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToAvro;

impl PluginCommand for ToAvro {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars to-avro"
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
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars to-avro test.avro",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn get_compression(call: &EvaluatedCall) -> Result<Option<AvroCompression>, ShellError> {
    if let Some((compression, span)) = call
        .get_flag_value("compression")
        .map(|e| e.as_str().map(|s| (s.to_owned(), e.span())))
        .transpose()?
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
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let file_name: Spanned<PathBuf> = call.req(0)?;
    let file_path = expand_path_with(&file_name.item, engine.get_current_dir()?, true);
    let compression = get_compression(call)?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_name.span),
        help: None,
        inner: vec![],
    })?;

    AvroWriter::new(file)
        .with_compression(compression)
        .finish(&mut df.to_polars())
        .map_err(|e| ShellError::GenericError {
            error: "Error saving file".into(),
            msg: e.to_string(),
            span: Some(file_name.span),
            help: None,
            inner: vec![],
        })?;

    let file_value = Value::string(format!("saved {:?}", &file_name.item), file_name.span);

    Ok(PipelineData::Value(
        Value::list(vec![file_value], call.head),
        None,
    ))
}

#[cfg(test)]
pub mod test {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{Span, Value};
    use uuid::Uuid;

    use crate::PolarsPlugin;

    #[test]
    pub fn test_to_avro() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        tmp_file.push(format!("{}.avro", Uuid::new_v4()));
        let tmp_file_str = tmp_file.to_str().expect("should be able to get file path");

        let cmd = format!(
            "[[a b]; [1 2] [3 4]] | polars into-df | polars to-avro {}",
            tmp_file_str
        );
        let mut plugin_test = PluginTest::new("polars", PolarsPlugin::default().into())?;
        plugin_test.engine_state_mut().add_env_var(
            "PWD".to_string(),
            Value::string(
                tmp_dir
                    .path()
                    .to_str()
                    .expect("should be able to get path")
                    .to_owned(),
                Span::test_data(),
            ),
        );
        let pipeline_data = plugin_test.eval(&cmd)?;

        assert!(tmp_file.exists());

        let value = pipeline_data.into_value(Span::test_data());
        let list = value.as_list()?;
        assert_eq!(list.len(), 1);
        let msg = list.first().expect("should have a value").as_str()?;
        assert!(msg.contains("saved"));
        Ok(())
    }
}
