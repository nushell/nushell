//! Serde mapping for plugin protocol (`protocol_info`).

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
struct ProtocolInfoRef<'a> {
    protocol: &'a Protocol,
    version: &'a str,
    features: &'a Vec<Feature>,
}

#[derive(Deserialize)]
struct ProtocolInfoDef {
    protocol: Protocol,
    version: String,
    features: Vec<Feature>,
}

#[derive(Serialize)]
enum ProtocolRef {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

#[derive(Deserialize)]
enum ProtocolDef {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

#[derive(Serialize)]
#[serde(tag = "name")]
enum FeatureRef {
    LocalSocket,
}

#[derive(Deserialize)]
#[serde(tag = "name")]
enum FeatureDef {
    LocalSocket,
    #[serde(other)]
    Unknown,
}

impl Serialize for ProtocolInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ProtocolInfoRef {
            protocol: &self.protocol,
            version: &self.version,
            features: &self.features,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProtocolInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ProtocolInfoDef::deserialize(deserializer)?;
        Ok(Self {
            protocol: def.protocol,
            version: def.version,
            features: def.features,
        })
    }
}

impl Serialize for Protocol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::NuPlugin => ProtocolRef::NuPlugin.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Protocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match ProtocolDef::deserialize(deserializer)? {
            ProtocolDef::NuPlugin => Self::NuPlugin,
        })
    }
}

impl Serialize for Feature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::LocalSocket => FeatureRef::LocalSocket.serialize(serializer),
            Self::Unknown => Err(serde::ser::Error::custom(
                "cannot serialize unknown protocol feature",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Feature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match FeatureDef::deserialize(deserializer)? {
            FeatureDef::LocalSocket => Self::LocalSocket,
            FeatureDef::Unknown => Self::Unknown,
        })
    }
}
