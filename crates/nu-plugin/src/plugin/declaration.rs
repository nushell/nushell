use crate::{EncodingType, EvaluatedCall};

use super::{call_plugin, create_command};
use crate::protocol::{
    CallInfo, CallInput, PluginCall, PluginCustomValue, PluginData, PluginResponse,
};
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
        let input = match input {
            Value::CustomValue { val, span } => {
                match val.as_any().downcast_ref::<PluginCustomValue>() {
                    Some(plugin_data) if plugin_data.filename == self.filename => {
                        CallInput::Data(PluginData {
                            data: plugin_data.data.clone(),
                            span,
                        })
                    }
                    _ => {
                        let custom_value_name = val.value_string();
                        return Err(ShellError::GenericError(
                            format!(
                                "Plugin {} can not handle the custom value {}",
                                self.name, custom_value_name
                            ),
                            format!("custom value {}", custom_value_name),
                            Some(span),
                            None,
                            Vec::new(),
                        ));
                    }
                }
            }
            value => CallInput::Value(value),
        };

        let plugin_call = PluginCall::CallInfo(CallInfo {
            name: self.name.clone(),
            call: EvaluatedCall::try_from_call(call, engine_state, stack)?,
            input,
        });

        let response =
            call_plugin(&mut child, plugin_call, &self.encoding, call.head).map_err(|err| {
                let decl = engine_state.get_decl(call.decl_id);
                ShellError::GenericError(
                    format!("Unable to decode call for {}", decl.name()),
                    err.to_string(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            });

        let pipeline_data = match response {
            Ok(PluginResponse::Value(value)) => {
                Ok(PipelineData::Value(value.as_ref().clone(), None))
            }
            Ok(PluginResponse::PluginData(name, plugin_data)) => Ok(PipelineData::Value(
                Value::CustomValue {
                    val: Box::new(PluginCustomValue {
                        name,
                        data: plugin_data.data,
                        filename: self.filename.clone(),
                        shell: self.shell.clone(),
                        encoding: self.encoding.clone(),
                        source: engine_state.get_decl(call.decl_id).name().to_owned(),
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
        };

        // We need to call .wait() on the child, or we'll risk summoning the zombie horde
        let _ = child.wait();

        pipeline_data
    }

    fn is_plugin(&self) -> Option<(&PathBuf, &str, &Option<PathBuf>)> {
        Some((&self.filename, self.encoding.to_str(), &self.shell))
    }
}
