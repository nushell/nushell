use crate::{EncodingType, EvaluatedCall};

use super::{create_command, OUTPUT_BUFFER_SIZE};
use crate::protocol::{
    CallInfo, CallInput, PluginCall, PluginCustomValue, PluginData, PluginResponse,
};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, Signature};
use nu_protocol::{PipelineData, ShellError, Value};

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
            ShellError::GenericError(
                format!("Unable to spawn plugin for {}", decl.name()),
                format!("{}", err),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?;

        let input = input.into_value(call.head);

        // Create message to plugin to indicate that signature is required and
        // send call to plugin asking for signature
        if let Some(mut stdin_writer) = child.stdin.take() {
            let encoding_clone = self.encoding.clone();
            let input = match input {
                Value::CustomValue { val, span } => {
                    match val.as_any().downcast_ref::<PluginCustomValue>() {
                        Some(plugin_data) => CallInput::Data(PluginData {
                            data: plugin_data.data.clone(),
                            span,
                        }),
                        // TODO: sending random custom values to plugins is probably never the right
                        // thing to do, we should probably just collapse them here and send base values
                        // For example what will a plugin do with an SQLiteDatabase?
                        None => CallInput::Value(Value::CustomValue { val, span }),
                    }
                }
                value => CallInput::Value(value),
            };

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

            let response = self.encoding.decode_response(&mut buf_read).map_err(|err| {
                let decl = engine_state.get_decl(call.decl_id);
                ShellError::GenericError(
                    format!("Unable to decode call for {}", decl.name()),
                    err.to_string(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            });

            match response {
                Ok(PluginResponse::Value(value)) => {
                    Ok(PipelineData::Value(value.as_ref().clone(), None))
                }
                Ok(PluginResponse::PluginData(plugin_data)) => Ok(PipelineData::Value(
                    Value::CustomValue {
                        val: Box::new(PluginCustomValue {
                            data: plugin_data.data,
                        }),
                        span: plugin_data.span,
                    },
                    None,
                )),
                Ok(PluginResponse::Error(err)) => Err(err.into()),
                Ok(PluginResponse::Signature(..)) => Err(ShellError::GenericError(
                    "Plugin missing value".into(),
                    "Received a signature from plugin instead of value".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )),
                Err(err) => Err(err),
            }
        } else {
            Err(ShellError::GenericError(
                "Error with stdout reader".into(),
                "no stdout reader".into(),
                Some(call.head),
                None,
                Vec::new(),
            ))
        };

        // We need to call .wait() on the child, or we'll risk summoning the zombie horde
        let _ = child.wait();

        pipeline_data
    }

    fn is_plugin(&self) -> Option<(&PathBuf, &str, &Option<PathBuf>)> {
        Some((&self.filename, self.encoding.to_str(), &self.shell))
    }
}
