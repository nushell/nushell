use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value};
use std::io::Write;

use std::path::Path;

#[derive(Clone)]
pub struct Save;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
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
            match engine_state.find_decl(format!("to {}", ext).as_bytes()) {
                Some(converter_id) => {
                    let output = engine_state.get_decl(converter_id).run(
                        engine_state,
                        stack,
                        &Call::new(),
                        input,
                    )?;

                    let output = output.into_value(span);

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
                }
                None => {
                    let output = input.collect_string("", &config)?;

                    if let Err(err) = file.write_all(output.as_bytes()) {
                        return Err(ShellError::IOError(err.to_string()));
                    }

                    Ok(PipelineData::new(span))
                }
            }
        } else {
            let output = input.collect_string("", &config)?;

            if let Err(err) = file.write_all(output.as_bytes()) {
                return Err(ShellError::IOError(err.to_string()));
            }

            Ok(PipelineData::new(span))
        }
    }
}
