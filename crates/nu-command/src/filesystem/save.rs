use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value};
use std::io::Write;

use std::path::Path;

#[derive(Clone)]
pub struct Save;

impl Command for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn usage(&self) -> &str {
        "Save a file."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("save")
            .required("filename", SyntaxShape::Filepath, "the filename to use")
            .switch("raw", "open file as raw binary", Some('r'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let raw = call.has_flag("raw");

        let span = call.head;

        let config = stack.get_config()?;

        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let arg_span = path.span;
        let path = Path::new(&path.item);

        let mut file = match std::fs::File::create(path) {
            Ok(file) => file,
            Err(err) => {
                return Ok(PipelineData::Value(
                    Value::Error {
                        error: ShellError::SpannedLabeledError(
                            "Permission denied".into(),
                            err.to_string(),
                            arg_span,
                        ),
                    },
                    None,
                ));
            }
        };

        let ext = if raw {
            None
        } else {
            path.extension()
                .map(|name| name.to_string_lossy().to_string())
        };

        if let Some(ext) = ext {
            let output = match engine_state.find_decl(format!("to {}", ext).as_bytes()) {
                Some(converter_id) => {
                    let output = engine_state.get_decl(converter_id).run(
                        engine_state,
                        stack,
                        &Call::new(span),
                        input,
                    )?;

                    output.into_value(span)
                }
                None => input.into_value(span),
            };

            match output {
                Value::String { val, .. } => {
                    if let Err(err) = file.write_all(val.as_bytes()) {
                        return Err(ShellError::IOError(err.to_string()));
                    }

                    Ok(PipelineData::new(span))
                }
                Value::Binary { val, .. } => {
                    if let Err(err) = file.write_all(&val) {
                        return Err(ShellError::IOError(err.to_string()));
                    }

                    Ok(PipelineData::new(span))
                }
                v => Err(ShellError::UnsupportedInput(v.get_type().to_string(), span)),
            }
        } else {
            match input.into_value() {
                Value::String { val, .. } => {
                    if let Err(err) = file.write_all(val.as_bytes()) {
                        return Err(ShellError::IOError(err.to_string()));
                    }

                    Ok(PipelineData::new(span))
                }
                Value::Binary { val, .. } => {
                    if let Err(err) = file.write_all(&val) {
                        return Err(ShellError::IOError(err.to_string()));
                    }

                    Ok(PipelineData::new(span))
                }
                v => Err(ShellError::UnsupportedInput(v.get_type().to_string(), span)),
            }

            Ok(PipelineData::new(span))
        }
    }
}
