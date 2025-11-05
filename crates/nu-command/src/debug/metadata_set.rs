use super::util::{extend_record_with_metadata, parse_metadata_from_record};
use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{DataSource, engine::Closure};

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
            .optional(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Record(vec![])])),
                "A closure that receives the current metadata and returns a new metadata record. Cannot be used with other flags.",
            )
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
            .named(
                "merge",
                SyntaxShape::Record(vec![]),
                "Merge arbitrary metadata fields",
                Some('m'),
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
        let closure: Option<Closure> = call.opt(engine_state, stack, 0)?;
        let ds_fp: Option<String> = call.get_flag(engine_state, stack, "datasource-filepath")?;
        let ds_ls = call.has_flag(engine_state, stack, "datasource-ls")?;
        let content_type: Option<String> = call.get_flag(engine_state, stack, "content-type")?;
        let merge: Option<Value> = call.get_flag(engine_state, stack, "merge")?;

        let mut metadata = match &mut input {
            PipelineData::Value(_, metadata)
            | PipelineData::ListStream(_, metadata)
            | PipelineData::ByteStream(_, metadata) => metadata.take().unwrap_or_default(),
            PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span: head }),
        };

        // Handle closure parameter - mutually exclusive with flags
        if let Some(closure) = closure {
            if ds_fp.is_some() || ds_ls || content_type.is_some() || merge.is_some() {
                return Err(ShellError::GenericError {
                    error: "Incompatible parameters".into(),
                    msg: "cannot use closure with other flags".into(),
                    span: Some(head),
                    help: Some("Use either the closure parameter or flags, not both".into()),
                    inner: vec![],
                });
            }

            let record = extend_record_with_metadata(Record::new(), Some(&metadata), head);
            let metadata_value = record.into_value(head);

            let result = ClosureEvalOnce::new(engine_state, stack, closure)
                .run_with_value(metadata_value)?
                .into_value(head)?;

            let result_record = result.as_record().map_err(|err| ShellError::GenericError {
                error: "Closure must return a record".into(),
                msg: format!("got {}", result.get_type()),
                span: Some(head),
                help: Some("The closure should return a record with metadata fields".into()),
                inner: vec![err],
            })?;

            metadata = parse_metadata_from_record(result_record);
            return Ok(input.set_metadata(Some(metadata)));
        }

        // Flag-based metadata modification
        if let Some(content_type) = content_type {
            metadata.content_type = Some(content_type);
        }

        if let Some(merge) = merge {
            let custom_record = merge.as_record()?;
            for (key, value) in custom_record {
                metadata.custom.insert(key.clone(), value.clone());
            }
        }

        match (ds_fp, ds_ls) {
            (Some(path), false) => metadata.data_source = DataSource::FilePath(path.into()),
            (None, true) => metadata.data_source = DataSource::Ls,
            (Some(_), true) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "cannot use `--datasource-filepath`".into(),
                    left_span: call
                        .get_flag_span(stack, "datasource-filepath")
                        .expect("has flag"),
                    right_message: "with `--datasource-ls`".into(),
                    right_span: call
                        .get_flag_span(stack, "datasource-ls")
                        .expect("has flag"),
                });
            }
            (None, false) => (),
        }

        Ok(input.set_metadata(Some(metadata)))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set the metadata of a table literal",
                example: "[[name color]; [Cargo.lock '#ff0000'] [Cargo.toml '#00ff00'] [README.md '#0000ff']] | metadata set --datasource-ls",
                result: None,
            },
            Example {
                description: "Set the metadata of a file path",
                example: "'crates' | metadata set --datasource-filepath $'(pwd)/crates'",
                result: None,
            },
            Example {
                description: "Set the content type metadata",
                example: "'crates' | metadata set --content-type text/plain | metadata | get content_type",
                result: Some(Value::test_string("text/plain")),
            },
            Example {
                description: "Set custom metadata",
                example: r#""data" | metadata set --merge {custom_key: "value"} | metadata | get custom_key"#,
                result: Some(Value::test_string("value")),
            },
            Example {
                description: "Set metadata using a closure",
                example: r#""data" | metadata set --content-type "text/csv" | metadata set {|m| $m | update content_type {$in + "-processed"}} | metadata | get content_type"#,
                result: Some(Value::test_string("text/csv-processed")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use crate::{Metadata, test_examples_with_commands};

    use super::*;

    #[test]
    fn test_examples() {
        test_examples_with_commands(MetadataSet {}, &[&Metadata {}])
    }
}
