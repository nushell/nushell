use nu_engine::command_prelude::*;
use nu_protocol::{DataSource, PipelineMetadata};

#[derive(Clone)]
pub struct MetadataSet;

impl Command for MetadataSet {
    fn name(&self) -> &str {
        "metadata set"
    }

    fn usage(&self) -> &str {
        "Set the metadata for items in the stream."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata set")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch(
                "datasource-ls",
                "Assign the DataSource::Ls metadata to the input",
                Some('l'),
            )
            .named(
                "datasource-filepath",
                SyntaxShape::Filepath,
                "Assign the DataSource::FilePath metadata to the input",
                Some('f'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let ds_fp: Option<String> = call.get_flag(engine_state, stack, "datasource-filepath")?;
        let ds_ls = call.has_flag(engine_state, stack, "datasource-ls")?;

        match (ds_fp, ds_ls) {
            (Some(path), false) => {
                let metadata = PipelineMetadata {
                    data_source: DataSource::FilePath(path.into()),
                };
                Ok(input.into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
            }
            (None, true) => {
                let metadata = PipelineMetadata {
                    data_source: DataSource::Ls,
                };
                Ok(input.into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
            }
            _ => Err(ShellError::IncorrectValue {
                msg: "Expected either --datasource-ls(-l) or --datasource-filepath(-f)".to_string(),
                val_span: head,
                call_span: head,
            }),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set the metadata of a table literal",
                example: "[[name color]; [Cargo.lock '#ff0000'] [Cargo.toml '#00ff00'] [README.md '#0000ff']] | metadata set --datasource-ls",
                result: None,
            },
            Example {
                description: "Set the metadata of a file path",
                example: "'crates' | metadata set --datasource-filepath $'(pwd)/crates' | metadata",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MetadataSet {})
    }
}
