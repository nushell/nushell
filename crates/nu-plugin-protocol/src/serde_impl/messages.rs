//! Serde mapping for plugin protocol (`messages`).

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
enum PluginInputRef<'a> {
    Hello(&'a ProtocolInfo),
    Call(usize, &'a PluginCall<PipelineDataHeader>),
    Goodbye,
    EngineCallResponse(usize, &'a EngineCallResponse<PipelineDataHeader>),
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
    Signal(nu_protocol::SignalAction),
}

#[derive(Deserialize)]
enum PluginInputDef {
    Hello(ProtocolInfo),
    Call(usize, PluginCall<PipelineDataHeader>),
    Goodbye,
    EngineCallResponse(usize, EngineCallResponse<PipelineDataHeader>),
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
    Signal(nu_protocol::SignalAction),
}

#[derive(Serialize)]
enum PluginOptionRef {
    GcDisabled(bool),
}

#[derive(Deserialize)]
enum PluginOptionDef {
    GcDisabled(bool),
}

#[derive(Serialize)]
enum PluginOutputRef<'a> {
    Hello(&'a ProtocolInfo),
    Option(&'a PluginOption),
    CallResponse(usize, &'a PluginCallResponse<PipelineDataHeader>),
    EngineCall {
        context: usize,
        id: usize,
        call: &'a EngineCall<PipelineDataHeader>,
    },
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Deserialize)]
enum PluginOutputDef {
    Hello(ProtocolInfo),
    Option(PluginOption),
    CallResponse(usize, PluginCallResponse<PipelineDataHeader>),
    EngineCall {
        context: usize,
        id: usize,
        call: EngineCall<PipelineDataHeader>,
    },
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

impl Serialize for PluginInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Hello(info) => PluginInputRef::Hello(info).serialize(serializer),
            Self::Call(id, call) => PluginInputRef::Call(*id, call).serialize(serializer),
            Self::Goodbye => PluginInputRef::Goodbye.serialize(serializer),
            Self::EngineCallResponse(id, response) => {
                PluginInputRef::EngineCallResponse(*id, response).serialize(serializer)
            }
            Self::Data(id, data) => PluginInputRef::Data(*id, data).serialize(serializer),
            Self::End(id) => PluginInputRef::End(*id).serialize(serializer),
            Self::Drop(id) => PluginInputRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => PluginInputRef::Ack(*id).serialize(serializer),
            Self::Signal(signal) => PluginInputRef::Signal(*signal).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginInputDef::deserialize(deserializer)? {
            PluginInputDef::Hello(info) => Self::Hello(info),
            PluginInputDef::Call(id, call) => Self::Call(id, call),
            PluginInputDef::Goodbye => Self::Goodbye,
            PluginInputDef::EngineCallResponse(id, response) => {
                Self::EngineCallResponse(id, response)
            }
            PluginInputDef::Data(id, data) => Self::Data(id, data),
            PluginInputDef::End(id) => Self::End(id),
            PluginInputDef::Drop(id) => Self::Drop(id),
            PluginInputDef::Ack(id) => Self::Ack(id),
            PluginInputDef::Signal(signal) => Self::Signal(signal),
        })
    }
}

impl Serialize for PluginOption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::GcDisabled(value) => PluginOptionRef::GcDisabled(*value).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginOptionDef::deserialize(deserializer)? {
            PluginOptionDef::GcDisabled(value) => Self::GcDisabled(value),
        })
    }
}

impl Serialize for PluginOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Hello(info) => PluginOutputRef::Hello(info).serialize(serializer),
            Self::Option(option) => PluginOutputRef::Option(option).serialize(serializer),
            Self::CallResponse(id, response) => {
                PluginOutputRef::CallResponse(*id, response).serialize(serializer)
            }
            Self::EngineCall { context, id, call } => PluginOutputRef::EngineCall {
                context: *context,
                id: *id,
                call,
            }
            .serialize(serializer),
            Self::Data(id, data) => PluginOutputRef::Data(*id, data).serialize(serializer),
            Self::End(id) => PluginOutputRef::End(*id).serialize(serializer),
            Self::Drop(id) => PluginOutputRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => PluginOutputRef::Ack(*id).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginOutputDef::deserialize(deserializer)? {
            PluginOutputDef::Hello(info) => Self::Hello(info),
            PluginOutputDef::Option(option) => Self::Option(option),
            PluginOutputDef::CallResponse(id, response) => Self::CallResponse(id, response),
            PluginOutputDef::EngineCall { context, id, call } => {
                Self::EngineCall { context, id, call }
            }
            PluginOutputDef::Data(id, data) => Self::Data(id, data),
            PluginOutputDef::End(id) => Self::End(id),
            PluginOutputDef::Drop(id) => Self::Drop(id),
            PluginOutputDef::Ack(id) => Self::Ack(id),
        })
    }
}
