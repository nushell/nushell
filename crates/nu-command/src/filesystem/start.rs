use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use std::path::Path;

#[derive(Clone)]
pub struct Start;

impl Command for Start {
    fn name(&self) -> &str {
        "start"
    }

    fn usage(&self) -> &str {
        "Open a folder or file in the default application or viewer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "folder", "directory", "run", "open"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("start")
            .input_output_types(vec![(Type::Nothing, Type::Any), (Type::String, Type::Any)])
            .optional("filepath", SyntaxShape::Filepath, "the filepath to open")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let path = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        let path = {
            if let Some(path_val) = path {
                Some(Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path_val.item),
                    span: path_val.span,
                })
            } else {
                path
            }
        };

        let path = if let Some(path) = path {
            path
        } else {
            // Collect a filename from the input
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {
                    return Err(ShellError::MissingParameter(
                        "needs filename".to_string(),
                        call.head,
                    ))
                }
                PipelineData::Value(val, ..) => val.as_spanned_string()?,
                _ => {
                    return Err(ShellError::MissingParameter(
                        "needs filename".to_string(),
                        call.head,
                    ));
                }
            }
        };
        let path_no_whitespace = &path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        let path = Path::new(path_no_whitespace);

        open::that(path)?;
        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Open a file, with structure (based on file extension or SQLite database header)",
                example: "open myfile.json",
                result: None,
            },
            Example {
                description: "Open a file, as raw bytes",
                example: "open myfile.json --raw",
                result: None,
            },
            Example {
                description: "Open a file, using the input to get filename",
                example: "'myfile.txt' | open",
                result: None,
            },
            Example {
                description: "Open a file, and decode it by the specified encoding",
                example: "open myfile.txt --raw | decode utf-8",
                result: None,
            },
        ]
    }
}
