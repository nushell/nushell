use nu_engine::command_prelude::*;

const NO_SPAN: Span = Span::unknown();

#[derive(Clone)]
pub struct MimeList;

impl Command for MimeList {
    fn name(&self) -> &str {
        "mime list"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .required(
                "mime_str",
                SyntaxShape::String,
                r#"Mime type to find extensions for. Format is <main type>/<subtype>.
<subtype> can be "*" to find all extensions for the <main type>.
If <main type> is "*" all known extensions are returned."#,
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Get a list of known extensions for a MIME type string."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: r#"mime list "video/mp4""#,
                description: r#"Get known extensions for the "video/mp4" mime type"#,
                result: Some(Value::list(
                    mime_guess::get_mime_extensions_str("video/mp4")
                        .expect("failed getting video/mp4 extensions")
                        .iter()
                        .map(|s| Value::string(s.to_string(), NO_SPAN))
                        .collect(),
                    NO_SPAN,
                )),
            },
            Example {
                example: r#"mime list "video/*""#,
                description: "Get all known video extensions",
                result: None,
            },
            Example {
                example: r#"mime list "*/whatever""#,
                description: "Get all known extensions",
                result: None,
            },
            Example {
                example: r#"mime list "nonexistent""#,
                description: r#"Unrecognized MIME types return an empty list"#,
                result: Some(Value::list(Vec::new(), NO_SPAN)),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mime_str: Spanned<String> = call.req(engine_state, stack, 0)?;

        let extensions = mime_guess::get_mime_extensions_str(&mime_str.item)
            .unwrap_or_default()
            .iter()
            .map(|ext| Value::string(ext.to_string(), NO_SPAN))
            .collect::<Vec<_>>();

        Ok(Value::list(extensions, mime_str.span).into_pipeline_data())
    }
}
