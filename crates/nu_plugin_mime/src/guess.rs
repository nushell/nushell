use nu_plugin::PluginCommand;
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Type, Value,
};

use crate::Mime;

pub struct MimeGuess;

impl PluginCommand for MimeGuess {
    type Plugin = Mime;

    fn name(&self) -> &str {
        "mime guess"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::Table(Box::new([
                        ("name".to_string(), Type::String),
                        ("type".to_string(), Type::String),
                    ])),
                ),
            ])
            .switch(
                "extension",
                "Accept extensions as input rather than file paths",
                Some('e'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Guess the MIME/Media Type of an extension or path. No disk access is performed."
    }

    fn extra_usage(&self) -> &str {
        r#"Because no disk access is performed, inputs that have no extensions, such as directory names, will return "unknown"."#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: r#""video.mkv" | mime guess"#,
                description: "Guess the MIME type from the path and return a string.",
                result: Some(Value::test_string("video/x-matroska")),
            },
            Example {
                example: r#"["video.mkv" "audio.mp3"] | mime guess"#,
                description: "Guess the MIME types from the paths and return a table.",
                result: Some(Value::test_list(vec![
                    Value::test_record(
                        record!("name" => Value::test_string("video.mkv"), "type" => Value::test_string("video/x-matroska")),
                    ),
                    Value::test_record(
                        record!("name" => Value::test_string("audio.mp3"), "type" => Value::test_string("audio/mpeg")),
                    ),
                ])),
            },
            Example {
                example: r#"["mkv" "mp3"] | mime guess -e"#,
                description: "Guess the MIME types from the extensions and return a table.",
                result: Some(Value::test_list(vec![
                    Value::test_record(
                        record!("name" => Value::test_string("mkv"), "type" => Value::test_string("video/x-matroska")),
                    ),
                    Value::test_record(
                        record!("name" => Value::test_string("mp3"), "type" => Value::test_string("audio/mpeg")),
                    ),
                ])),
            },
            Example {
                example: r#"let input = glob * | wrap filename; $input | merge ($input | get filename | mime guess | select type)"#,
                description: "Add a MIME type column to a table.",
                result: Some(Value::test_list(vec![Value::test_record(
                    record!("filename" => Value::test_string("..."), "type" => Value::test_string("...")),
                )])),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        let use_extension: bool = call.has_flag("extension")?;

        let guess_function: fn(&str) -> mime_guess::MimeGuess = if use_extension {
            mime_guess::from_ext
        } else {
            // HACK Not sure how to satisfy the compiler here without a closure, but we cannot return the function directly.
            // If we do, we get an error that the types are different or that a value does not live long enough when the function is called.
            |input| mime_guess::from_path(input)
        };

        match input {
            PipelineData::Value(Value::String { val, internal_span }, ..) => {
                let mime_type = guess_function(&val)
                    .first()
                    .map(|mime| mime.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                Ok(Value::string(mime_type, internal_span).into_pipeline_data())
            }
            PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream(..) => {
                let mime_records_iter = input.into_iter().map(move |value| {
                    let span = value.span();

                    match value.as_str() {
                        Ok(s) => {
                            let name = Value::string(s, span);
                            let mime_type = Value::string(
                                guess_function(s)
                                    .first()
                                    .map(|mime| mime.to_string())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                span,
                            );

                            Value::record(record!("name" => name, "type" => mime_type), span)
                        }
                        Err(err) => Value::error(err, span),
                    }
                });

                let ctrlc = compile_error!("can't figure out how to get ctrlc in plugin yet");

                Ok(mime_records_iter.into_pipeline_data(call.head, ctrlc))
            }
            PipelineData::Empty => Ok(PipelineData::empty()),
            _ => Err(ShellError::TypeMismatch {
                err_message: "Only string input is supported.".to_string(),
                span: input.span().unwrap_or(Span::unknown()),
            }
            .into()),
        }
    }
}
