use std::path::PathBuf;

use nu_protocol::{CustomValue, ShellError, Value};
use serde::Serialize;

use crate::plugin::{call_plugin, create_command, get_plugin_encoding};

use super::{PluginCall, PluginData, PluginResponse};

/// An opaque container for a custom value that is handled fully by a plugin
///
/// This is constructed by the main nushell engine when it receives [`PluginResponse::PluginData`]
/// it stores that data as well as metadata related to the plugin to be able to call the plugin
/// later.
/// Since the data in it is opaque to the engine, there are only two final destinations for it:
/// either it will be sent back to the plugin that generated it across a pipeline, or it will be
/// sent to the plugin with a request to collapse it into a base value
#[derive(Clone, Debug, Serialize)]
pub struct PluginCustomValue {
    /// The name of the custom value as defined by the plugin
    pub name: String,
    pub data: Vec<u8>,
    pub filename: PathBuf,

    // PluginCustomValue must implement Serialize because all CustomValues must implement Serialize
    // However, the main place where values are serialized and deserialized is when they are being
    // sent between plugins and nushell's main engine. PluginCustomValue is never meant to be sent
    // between that boundary
    #[serde(skip)]
    pub shell: Option<PathBuf>,
    #[serde(skip)]
    pub source: String,
}

impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> nu_protocol::Value {
        Value::CustomValue {
            val: Box::new(self.clone()),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.name.clone()
    }

    fn to_base_value(
        &self,
        span: nu_protocol::Span,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let mut plugin_cmd = create_command(&self.filename, &self.shell);

        let mut child = plugin_cmd.spawn().map_err(|err| {
            ShellError::GenericError(
                format!(
                    "Unable to spawn plugin for {} to get base value",
                    self.source
                ),
                format!("{}", err),
                Some(span),
                None,
                Vec::new(),
            )
        })?;

        let plugin_call = PluginCall::CollapseCustomValue(PluginData {
            data: self.data.clone(),
            span,
        });
        let encoding = {
            let stdout_reader = match &mut child.stdout {
                Some(out) => out,
                None => {
                    return Err(ShellError::PluginFailedToLoad(
                        "Plugin missing stdout reader".into(),
                    ))
                }
            };
            get_plugin_encoding(stdout_reader)?
        };

        let response = call_plugin(&mut child, plugin_call, &encoding, span).map_err(|err| {
            ShellError::GenericError(
                format!(
                    "Unable to decode call for {} to get base value",
                    self.source
                ),
                format!("{}", err),
                Some(span),
                None,
                Vec::new(),
            )
        });

        let value = match response {
            Ok(PluginResponse::Value(value)) => Ok(*value),
            Ok(PluginResponse::PluginData(..)) => Err(ShellError::GenericError(
                "Plugin misbehaving".into(),
                "Plugin returned custom data as a response to a collapse call".into(),
                Some(span),
                None,
                Vec::new(),
            )),
            Ok(PluginResponse::Error(err)) => Err(err.into()),
            Ok(PluginResponse::Signature(..)) => Err(ShellError::GenericError(
                "Plugin missing value".into(),
                "Received a signature from plugin instead of value".into(),
                Some(span),
                None,
                Vec::new(),
            )),
            Err(err) => Err(err),
        };

        // We need to call .wait() on the child, or we'll risk summoning the zombie horde
        let _ = child.wait();

        value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn typetag_name(&self) -> &'static str {
        "PluginCustomValue"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}
