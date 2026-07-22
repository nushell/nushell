//! Serde mapping for plugin protocol (`stream`).

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
enum StreamDataRef<'a> {
    List(&'a nu_protocol::Value),
    Raw(&'a Result<Vec<u8>, LabeledError>),
}

#[derive(Deserialize)]
enum StreamDataDef {
    List(nu_protocol::Value),
    Raw(Result<Vec<u8>, LabeledError>),
}

#[derive(Serialize)]
enum StreamMessageRef<'a> {
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Deserialize)]
enum StreamMessageDef {
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

impl Serialize for StreamData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::List(value) => StreamDataRef::List(value).serialize(serializer),
            Self::Raw(value) => StreamDataRef::Raw(value).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StreamData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match StreamDataDef::deserialize(deserializer)? {
            StreamDataDef::List(value) => Self::List(value),
            StreamDataDef::Raw(value) => Self::Raw(value),
        })
    }
}

impl Serialize for StreamMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Data(id, data) => StreamMessageRef::Data(*id, data).serialize(serializer),
            Self::End(id) => StreamMessageRef::End(*id).serialize(serializer),
            Self::Drop(id) => StreamMessageRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => StreamMessageRef::Ack(*id).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StreamMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match StreamMessageDef::deserialize(deserializer)? {
            StreamMessageDef::Data(id, data) => Self::Data(id, data),
            StreamMessageDef::End(id) => Self::End(id),
            StreamMessageDef::Drop(id) => Self::Drop(id),
            StreamMessageDef::Ack(id) => Self::Ack(id),
        })
    }
}
