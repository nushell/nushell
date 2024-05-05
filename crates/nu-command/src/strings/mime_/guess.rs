use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MimeGuess;

impl Command for MimeGuess {
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let use_extension: bool = call.has_flag(engine_state, stack, "extension")?;

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

                let ctrlc = engine_state.ctrlc.clone();

                Ok(mime_records_iter.into_pipeline_data(ctrlc))
            }
            PipelineData::Empty => Ok(PipelineData::empty()),
            _ => Err(ShellError::TypeMismatch {
                err_message: "Only string input is supported.".to_string(),
                span: input.span().unwrap_or(Span::unknown()),
            }),
        }
    }
}
