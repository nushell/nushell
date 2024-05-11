use nu_plugin::SimplePluginCommand;
use nu_protocol::{Category, Example, Signature, Spanned, SyntaxShape, Type, Value};

use crate::Mime;

pub struct MimeList;

impl SimplePluginCommand for MimeList {
    type Plugin = Mime;

    fn name(&self) -> &str {
        "mime list"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::String, Type::List(Box::new(Type::String)))
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
                example: r#"mime list "video/x-matroska""#,
                description: r#"Get known extensions for the "video/x-matroska" mime type"#,
                result: Some(Value::test_list(
                    mime_guess::get_mime_extensions_str("video/x-matroska")
                        .expect("failed getting video/x-matroska extensions")
                        .iter()
                        .map(|s| Value::test_string(*s))
                        .collect(),
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
                result: Some(Value::test_list(Vec::new())),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        let mime_str: Spanned<String> = call.req(0)?;

        let extensions = mime_guess::get_mime_extensions_str(&mime_str.item)
            .unwrap_or_default()
            .iter()
            .map(|ext| Value::string(*ext, mime_str.span))
            .collect::<Vec<_>>();

        Ok(Value::list(extensions, mime_str.span))
    }
}
