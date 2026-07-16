//! Serde mapping for plugin protocol (`pipeline`).

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
enum PipelineDataHeaderRef<'a> {
    Empty,
    Value(
        &'a nu_protocol::Value,
        &'a Option<nu_protocol::PipelineMetadata>,
    ),
    ListStream(&'a ListStreamInfo),
    ByteStream(&'a ByteStreamInfo),
}

#[derive(Deserialize)]
enum PipelineDataHeaderDef {
    Empty,
    Value(nu_protocol::Value, Option<nu_protocol::PipelineMetadata>),
    ListStream(ListStreamInfo),
    ByteStream(ByteStreamInfo),
}

#[derive(Serialize)]
struct ListStreamInfoRef {
    id: usize,
    span: nu_protocol::Span,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Deserialize)]
struct ListStreamInfoDef {
    id: usize,
    span: nu_protocol::Span,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Serialize)]
struct ByteStreamInfoRef {
    id: usize,
    span: nu_protocol::Span,
    #[serde(rename = "type")]
    type_: nu_protocol::ByteStreamType,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Deserialize)]
struct ByteStreamInfoDef {
    id: usize,
    span: nu_protocol::Span,
    #[serde(rename = "type")]
    type_: nu_protocol::ByteStreamType,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

impl Serialize for PipelineDataHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Empty => PipelineDataHeaderRef::Empty.serialize(serializer),
            Self::Value(value, metadata) => {
                PipelineDataHeaderRef::Value(value, metadata).serialize(serializer)
            }
            Self::ListStream(info) => PipelineDataHeaderRef::ListStream(info).serialize(serializer),
            Self::ByteStream(info) => PipelineDataHeaderRef::ByteStream(info).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PipelineDataHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PipelineDataHeaderDef::deserialize(deserializer)? {
            PipelineDataHeaderDef::Empty => Self::Empty,
            PipelineDataHeaderDef::Value(value, metadata) => Self::Value(value, metadata),
            PipelineDataHeaderDef::ListStream(info) => Self::ListStream(info),
            PipelineDataHeaderDef::ByteStream(info) => Self::ByteStream(info),
        })
    }
}

impl Serialize for ListStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ListStreamInfoRef {
            id: self.id,
            span: self.span,
            metadata: self.metadata.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ListStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ListStreamInfoDef::deserialize(deserializer)?;
        Ok(Self {
            id: def.id,
            span: def.span,
            metadata: def.metadata,
        })
    }
}

impl Serialize for ByteStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ByteStreamInfoRef {
            id: self.id,
            span: self.span,
            type_: self.type_,
            metadata: self.metadata.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ByteStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ByteStreamInfoDef::deserialize(deserializer)?;
        Ok(Self {
            id: def.id,
            span: def.span,
            type_: def.type_,
            metadata: def.metadata,
        })
    }
}
