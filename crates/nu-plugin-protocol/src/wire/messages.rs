//! Wire mapping for top-level plugin input/output messages.

use crate::{
    EngineCall, EngineCallResponse, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginInput, PluginOption, PluginOutput, ProtocolInfo, StreamData,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
enum WirePluginInput<'a> {
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
enum WirePluginInputDe {
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

impl<'a> From<&'a PluginInput> for WirePluginInput<'a> {
    fn from(input: &'a PluginInput) -> Self {
        match input {
            PluginInput::Hello(info) => Self::Hello(info),
            PluginInput::Call(id, call) => Self::Call(*id, call),
            PluginInput::Goodbye => Self::Goodbye,
            PluginInput::EngineCallResponse(id, response) => {
                Self::EngineCallResponse(*id, response)
            }
            PluginInput::Data(id, data) => Self::Data(*id, data),
            PluginInput::End(id) => Self::End(*id),
            PluginInput::Drop(id) => Self::Drop(*id),
            PluginInput::Ack(id) => Self::Ack(*id),
            PluginInput::Signal(signal) => Self::Signal(*signal),
        }
    }
}

impl From<WirePluginInputDe> for PluginInput {
    fn from(wire: WirePluginInputDe) -> Self {
        match wire {
            WirePluginInputDe::Hello(info) => Self::Hello(info),
            WirePluginInputDe::Call(id, call) => Self::Call(id, call),
            WirePluginInputDe::Goodbye => Self::Goodbye,
            WirePluginInputDe::EngineCallResponse(id, response) => {
                Self::EngineCallResponse(id, response)
            }
            WirePluginInputDe::Data(id, data) => Self::Data(id, data),
            WirePluginInputDe::End(id) => Self::End(id),
            WirePluginInputDe::Drop(id) => Self::Drop(id),
            WirePluginInputDe::Ack(id) => Self::Ack(id),
            WirePluginInputDe::Signal(signal) => Self::Signal(signal),
        }
    }
}

#[derive(Serialize, Deserialize)]
enum WirePluginOption {
    GcDisabled(bool),
}

impl From<&PluginOption> for WirePluginOption {
    fn from(option: &PluginOption) -> Self {
        match option {
            PluginOption::GcDisabled(value) => Self::GcDisabled(*value),
        }
    }
}

impl From<WirePluginOption> for PluginOption {
    fn from(wire: WirePluginOption) -> Self {
        match wire {
            WirePluginOption::GcDisabled(value) => Self::GcDisabled(value),
        }
    }
}

#[derive(Serialize)]
enum WirePluginOutput<'a> {
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
enum WirePluginOutputDe {
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

impl<'a> From<&'a PluginOutput> for WirePluginOutput<'a> {
    fn from(output: &'a PluginOutput) -> Self {
        match output {
            PluginOutput::Hello(info) => Self::Hello(info),
            PluginOutput::Option(option) => Self::Option(option),
            PluginOutput::CallResponse(id, response) => Self::CallResponse(*id, response),
            PluginOutput::EngineCall { context, id, call } => Self::EngineCall {
                context: *context,
                id: *id,
                call,
            },
            PluginOutput::Data(id, data) => Self::Data(*id, data),
            PluginOutput::End(id) => Self::End(*id),
            PluginOutput::Drop(id) => Self::Drop(*id),
            PluginOutput::Ack(id) => Self::Ack(*id),
        }
    }
}

impl From<WirePluginOutputDe> for PluginOutput {
    fn from(wire: WirePluginOutputDe) -> Self {
        match wire {
            WirePluginOutputDe::Hello(info) => Self::Hello(info),
            WirePluginOutputDe::Option(option) => Self::Option(option),
            WirePluginOutputDe::CallResponse(id, response) => Self::CallResponse(id, response),
            WirePluginOutputDe::EngineCall { context, id, call } => {
                Self::EngineCall { context, id, call }
            }
            WirePluginOutputDe::Data(id, data) => Self::Data(id, data),
            WirePluginOutputDe::End(id) => Self::End(id),
            WirePluginOutputDe::Drop(id) => Self::Drop(id),
            WirePluginOutputDe::Ack(id) => Self::Ack(id),
        }
    }
}

impl Serialize for PluginInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginInput::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PluginInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WirePluginInputDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for PluginOption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginOption::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PluginOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WirePluginOption::deserialize(deserializer)?.into())
    }
}

impl Serialize for PluginOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginOutput::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PluginOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WirePluginOutputDe::deserialize(deserializer)?.into())
    }
}
