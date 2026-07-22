//! Serde mapping for plugin protocol (`evaluated`).

#![allow(unused_imports)]

use crate::{
    ByteStreamInfo, CallInfo, CustomValueOp, DynamicCompletionCall, EngineCall, EngineCallResponse,
    EvaluatedCall, Feature, GetCompletionArgType, GetCompletionInfo, ListStreamInfo, Ordering,
    PipelineDataHeader, PluginCall, PluginCallResponse, PluginCustomValue, PluginInput,
    PluginOption, PluginOutput, Protocol, ProtocolInfo, StreamData, StreamMessage,
};
use nu_protocol::{LabeledError, ShellError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
struct EvaluatedCallRef<'a> {
    head: nu_protocol::Span,
    positional: &'a Vec<nu_protocol::Value>,
    named: &'a Vec<(nu_protocol::Spanned<String>, Option<nu_protocol::Value>)>,
}

#[derive(Deserialize)]
struct EvaluatedCallDef {
    head: nu_protocol::Span,
    positional: Vec<nu_protocol::Value>,
    named: Vec<(nu_protocol::Spanned<String>, Option<nu_protocol::Value>)>,
}

#[derive(Serialize)]
struct PluginCustomValueRef<'a> {
    name: &'a str,
    data: &'a [u8],
    #[serde(default, skip_serializing_if = "is_false")]
    notify_on_drop: bool,
}

#[derive(Deserialize)]
struct PluginCustomValueDef {
    name: String,
    data: Vec<u8>,
    #[serde(default)]
    notify_on_drop: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl Serialize for EvaluatedCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EvaluatedCallRef {
            head: self.head,
            positional: &self.positional,
            named: &self.named,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EvaluatedCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = EvaluatedCallDef::deserialize(deserializer)?;
        Ok(Self {
            head: def.head,
            positional: def.positional,
            named: def.named,
        })
    }
}

impl Serialize for PluginCustomValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PluginCustomValueRef {
            name: self.name(),
            data: self.data(),
            notify_on_drop: self.notify_on_drop(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PluginCustomValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = PluginCustomValueDef::deserialize(deserializer)?;
        Ok(Self::new(def.name, def.data, def.notify_on_drop))
    }
}
