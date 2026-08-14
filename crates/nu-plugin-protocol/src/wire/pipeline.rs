//! Wire mapping for pipeline headers and stream info.

use crate::{ByteStreamInfo, ListStreamInfo, PipelineDataHeader};
use nu_protocol::{ByteStreamType, PipelineMetadata, Span, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::value::WireValue;

#[derive(Serialize)]
enum WirePipelineDataHeader {
    Empty,
    Value(WireValue, Option<PipelineMetadata>),
    ListStream(WireListStreamInfo),
    ByteStream(WireByteStreamInfo),
}

#[derive(Deserialize)]
enum WirePipelineDataHeaderDe {
    Empty,
    Value(WireValue, Option<PipelineMetadata>),
    ListStream(WireListStreamInfo),
    ByteStream(WireByteStreamInfo),
}

impl From<&PipelineDataHeader> for WirePipelineDataHeader {
    fn from(header: &PipelineDataHeader) -> Self {
        match header {
            PipelineDataHeader::Empty => Self::Empty,
            PipelineDataHeader::Value(value, metadata) => {
                Self::Value(WireValue::from(value), metadata.clone())
            }
            PipelineDataHeader::ListStream(info) => {
                Self::ListStream(WireListStreamInfo::from(info))
            }
            PipelineDataHeader::ByteStream(info) => {
                Self::ByteStream(WireByteStreamInfo::from(info))
            }
        }
    }
}

impl From<WirePipelineDataHeaderDe> for PipelineDataHeader {
    fn from(wire: WirePipelineDataHeaderDe) -> Self {
        match wire {
            WirePipelineDataHeaderDe::Empty => Self::Empty,
            WirePipelineDataHeaderDe::Value(value, metadata) => {
                Self::Value(Value::from(value), metadata)
            }
            WirePipelineDataHeaderDe::ListStream(info) => Self::ListStream(info.into()),
            WirePipelineDataHeaderDe::ByteStream(info) => Self::ByteStream(info.into()),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct WireListStreamInfo {
    id: usize,
    span: Span,
    metadata: Option<PipelineMetadata>,
}

impl From<&ListStreamInfo> for WireListStreamInfo {
    fn from(info: &ListStreamInfo) -> Self {
        Self {
            id: info.id,
            span: info.span,
            metadata: info.metadata.clone(),
        }
    }
}

impl From<WireListStreamInfo> for ListStreamInfo {
    fn from(wire: WireListStreamInfo) -> Self {
        Self {
            id: wire.id,
            span: wire.span,
            metadata: wire.metadata,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct WireByteStreamInfo {
    id: usize,
    span: Span,
    #[serde(rename = "type")]
    type_: ByteStreamType,
    metadata: Option<PipelineMetadata>,
}

impl From<&ByteStreamInfo> for WireByteStreamInfo {
    fn from(info: &ByteStreamInfo) -> Self {
        Self {
            id: info.id,
            span: info.span,
            type_: info.type_,
            metadata: info.metadata.clone(),
        }
    }
}

impl From<WireByteStreamInfo> for ByteStreamInfo {
    fn from(wire: WireByteStreamInfo) -> Self {
        Self {
            id: wire.id,
            span: wire.span,
            type_: wire.type_,
            metadata: wire.metadata,
        }
    }
}

impl Serialize for PipelineDataHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePipelineDataHeader::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PipelineDataHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WirePipelineDataHeaderDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for ListStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireListStreamInfo::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ListStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireListStreamInfo::deserialize(deserializer)?.into())
    }
}

impl Serialize for ByteStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireByteStreamInfo::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ByteStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireByteStreamInfo::deserialize(deserializer)?.into())
    }
}
