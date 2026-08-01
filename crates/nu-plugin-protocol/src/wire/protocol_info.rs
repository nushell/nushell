//! Wire mapping for protocol handshake types.

use crate::{Feature, Protocol, ProtocolInfo};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
struct WireProtocolInfo<'a> {
    protocol: WireProtocol,
    version: &'a str,
    /// Uses [`Feature`]'s Serialize so `Unknown` still fails to serialize.
    features: &'a [Feature],
}

#[derive(Deserialize)]
struct WireProtocolInfoDe {
    protocol: WireProtocol,
    version: String,
    features: Vec<Feature>,
}

impl<'a> From<&'a ProtocolInfo> for WireProtocolInfo<'a> {
    fn from(info: &'a ProtocolInfo) -> Self {
        Self {
            protocol: WireProtocol::from(&info.protocol),
            version: &info.version,
            features: &info.features,
        }
    }
}

impl From<WireProtocolInfoDe> for ProtocolInfo {
    fn from(wire: WireProtocolInfoDe) -> Self {
        Self {
            protocol: wire.protocol.into(),
            version: wire.version,
            features: wire.features,
        }
    }
}

#[derive(Serialize, Deserialize)]
enum WireProtocol {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

impl From<&Protocol> for WireProtocol {
    fn from(protocol: &Protocol) -> Self {
        match protocol {
            Protocol::NuPlugin => Self::NuPlugin,
        }
    }
}

impl From<WireProtocol> for Protocol {
    fn from(wire: WireProtocol) -> Self {
        match wire {
            WireProtocol::NuPlugin => Self::NuPlugin,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "name")]
enum WireFeature {
    LocalSocket,
    #[serde(other)]
    Unknown,
}

impl From<&Feature> for WireFeature {
    fn from(feature: &Feature) -> Self {
        match feature {
            Feature::LocalSocket => Self::LocalSocket,
            Feature::Unknown => Self::Unknown,
        }
    }
}

impl From<WireFeature> for Feature {
    fn from(wire: WireFeature) -> Self {
        match wire {
            WireFeature::LocalSocket => Self::LocalSocket,
            WireFeature::Unknown => Self::Unknown,
        }
    }
}

impl Serialize for ProtocolInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireProtocolInfo::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProtocolInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireProtocolInfoDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for Protocol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireProtocol::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Protocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireProtocol::deserialize(deserializer)?.into())
    }
}

impl Serialize for Feature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::LocalSocket => WireFeature::LocalSocket.serialize(serializer),
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
        Ok(WireFeature::deserialize(deserializer)?.into())
    }
}
