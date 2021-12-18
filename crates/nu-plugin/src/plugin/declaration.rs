use crate::{EncodingType, EvaluatedCall};

use super::{create_command, OUTPUT_BUFFER_SIZE};
use crate::protocol::{CallInfo, PluginCall, PluginResponse};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, Signature, Value};
use nu_protocol::{PipelineData, ShellError};

#[derive(Clone)]
pub struct PluginDeclaration {
    name: String,
    signature: Signature,
    filename: PathBuf,
    shell: Option<PathBuf>,
    encoding: EncodingType,
}

impl PluginDeclaration {
    pub fn new(
        filename: PathBuf,
        signature: Signature,
        encoding: EncodingType,
        shell: Option<PathBuf>,
    ) -> Self {
        Self {
            name: signature.name.clone(),
            signature,
            filename,
            encoding,
            shell,
        }
    }
}

impl Command for PluginDeclaration {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.signature.clone()
    }

    fn usage(&self) -> &str {
        self.signature.usage.as_str()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Call the command with self path
        // Decode information from plugin
        // Create PipelineData
        let source_file = Path::new(&self.filename);
        let mut plugin_cmd = create_command(source_file, &self.shell);

        let mut child = plugin_cmd.spawn().map_err(|err| {
            let decl = engine_state.get_decl(call.decl_id);
            ShellError::SpannedLabeledError(
                format!("Unable to spawn plugin for {}", decl.name()),
                format!("{}", err),
                call.head,
            )
        })?;

        let input = match input {
            PipelineData::Value(value, ..) => value,
            PipelineData::Stream(stream, ..) => {
                let values = stream.collect::<Vec<Value>>();

                Value::List {
                    vals: values,
                    span: call.head,
                }
            }
        };

        // Create message to plugin to indicate that signature is required and
        // send call to plugin asking for signature
        if let Some(mut stdin_writer) = child.stdin.take() {
            let encoding_clone = self.encoding.clone();
            let plugin_call = PluginCall::CallInfo(Box::new(CallInfo {
                name: self.name.clone(),
                call: EvaluatedCall::try_from_call(call, engine_state, stack)?,
                input,
            }));
            std::thread::spawn(move || {
                // PluginCall information
                encoding_clone.encode_call(&plugin_call, &mut stdin_writer)
            });
        }

        // Deserialize response from plugin to extract the resulting value
        let pipeline_data = if let Some(stdout_reader) = &mut child.stdout {
            let reader = stdout_reader;
            let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);

            let response = self
                .encoding
                .decode_response(&mut buf_read)
                .map_err(|err| {
                    let decl = engine_state.get_decl(call.decl_id);
                    ShellError::SpannedLabeledError(
                        format!("Unable to decode call for {}", decl.name()),
                        err.to_string(),
                        call.head,
                    )
                })?;

            match response {
                PluginResponse::Value(value) => {
                    Ok(PipelineData::Value(value.as_ref().clone(), None))
                }
                PluginResponse::Error(err) => Err(err.into()),
                PluginResponse::Signature(..) => Err(ShellError::SpannedLabeledError(
                    "Plugin missing value".into(),
                    "Received a signature from plugin instead of value".into(),
                    call.head,
                )),
            }
        } else {
            Err(ShellError::SpannedLabeledError(
                "Error with stdout reader".into(),
                "no stdout reader".into(),
                call.head,
            ))
        }?;

        // There is no need to wait for the child process to finish
        // The response has been collected from the plugin call
        Ok(pipeline_data)
    }

    fn is_plugin(&self) -> Option<(&PathBuf, &str, &Option<PathBuf>)> {
        Some((&self.filename, self.encoding.to_str(), &self.shell))
    }
}
