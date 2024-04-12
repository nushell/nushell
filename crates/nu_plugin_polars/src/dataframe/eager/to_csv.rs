use std::{fs::File, path::PathBuf};

use nu_path::expand_path_with;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value,
};
use polars::prelude::{CsvWriter, SerWriter};

use crate::PolarsPlugin;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToCSV;

impl PluginCommand for ToCSV {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars to-csv"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to CSV file."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .named(
                "delimiter",
                SyntaxShape::String,
                "file delimiter character",
                Some('d'),
            )
            .switch("no-header", "Indicates if file doesn't have header", None)
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Saves dataframe to CSV file",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv",
                result: None,
            },
            Example {
                description: "Saves dataframe to CSV file using other delimiter",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv --delimiter '|'",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(|e| e.into())
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
    let delimiter: Option<Spanned<String>> = call.get_flag("delimiter")?;
    let no_header: bool = call.has_flag("no-header")?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let mut file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_name.span),
        help: None,
        inner: vec![],
    })?;

    let writer = CsvWriter::new(&mut file);

    let writer = if no_header {
        writer.include_header(false)
    } else {
        writer.include_header(true)
    };

    let mut writer = match delimiter {
        None => writer,
        Some(d) => {
            if d.item.len() != 1 {
                return Err(ShellError::GenericError {
                    error: "Incorrect delimiter".into(),
                    msg: "Delimiter has to be one char".into(),
                    span: Some(d.span),
                    help: None,
                    inner: vec![],
                });
            } else {
                let delimiter = match d.item.chars().next() {
                    Some(d) => d as u8,
                    None => unreachable!(),
                };

                writer.with_separator(delimiter)
            }
        }
    };

    writer
        .finish(&mut df.to_polars())
        .map_err(|e| ShellError::GenericError {
            error: "Error writing to file".into(),
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
    pub fn test_to_csv() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        tmp_file.push(format!("{}.csv", Uuid::new_v4()));
        let tmp_file_str = tmp_file.to_str().expect("should be able to get file path");

        let cmd = format!(
            "[[a b]; [1 2] [3 4]] | polars into-df | polars to-csv {}",
            tmp_file_str
        );
        println!("cmd: {}", cmd);
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
