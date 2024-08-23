use nu_engine::command_prelude::*;
use nu_protocol::DataSource;

#[derive(Clone)]
pub struct MetadataSet;

impl Command for MetadataSet {
    fn name(&self) -> &str {
        "metadata set"
    }

    fn description(&self) -> &str {
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
            .named(
                "content-type",
                SyntaxShape::String,
                "Assign content type metadata to the input",
                Some('c'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let ds_fp: Option<String> = call.get_flag(engine_state, stack, "datasource-filepath")?;
        let ds_ls = call.has_flag(engine_state, stack, "datasource-ls")?;
        let content_type: Option<String> = call.get_flag(engine_state, stack, "content-type")?;

        let mut metadata = match &mut input {
            PipelineData::Value(_, metadata)
            | PipelineData::ListStream(_, metadata)
            | PipelineData::ByteStream(_, metadata) => metadata.take().unwrap_or_default(),
            PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span: head }),
        };

        if let Some(content_type) = content_type {
            metadata.content_type = Some(content_type);
        }

        match (ds_fp, ds_ls) {
            (Some(path), false) => metadata.data_source = DataSource::FilePath(path.into()),
            (None, true) => metadata.data_source = DataSource::Ls,
            (Some(_), true) => (), // TODO: error here
            (None, false) => (),
        }

        Ok(input.set_metadata(Some(metadata)))
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
            Example {
                description: "Set the metadata of a file path",
                example: "'crates' | metadata set --content-type text/plain | metadata",
                result: Some(Value::test_record(record! {
                    "content_type" => Value::test_string("text/plain"),
                })),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use crate::{test_examples_with_commands, Metadata};

    use super::*;

    #[test]
    fn test_examples() {
        test_examples_with_commands(MetadataSet {}, &[&Metadata {}])
    }
}
