use nu_protocol::ShellError;
use semver::Prerelease;
use serde::{Deserialize, Serialize};

/// Version of the plugin wire protocol, independent of the Nushell release version.
///
/// This is the compatibility contract exchanged in the `Hello` handshake. Two peers are
/// compatible when the higher version matches a semver caret requirement on the lower one. On
/// `0.x` that means the same minor line (`^0.1.0` is `>=0.1.0, <0.2.0`).
///
/// # What the version covers
///
/// Every type that crosses the wire falls into one of three tiers:
///
/// - **Frozen**: the messages defined in this crate, [`Value`](nu_protocol::Value) and the
///   `nu-protocol` types inside them (spans, errors as `LabeledError`, pipeline metadata,
///   signatures, types and syntax shapes, completion suggestions, plugin metadata, ...).
///   This crate's `src/wire_witness.rs` and `nu_protocol`'s `plugin/wire_witness.rs` list the
///   types with derived serde and stop the build when one of their variants or fields is
///   added, removed, or renamed. Field types, serde attributes, and the types with hand-written
///   serde are only checked by the snapshot corpus in `src/tests.rs`, and only for the values
///   the corpus holds, so add new variants and messages to it.
/// - **Tolerant**: the engine [`Config`](nu_protocol::Config). It travels as a record and is
///   parsed with the same code as `$env.config`, so added, removed, or unknown options never
///   break a peer and never need a version bump.
/// - **Engine-internal**: IR blocks (`GetBlockIR`) and the parsed call sent with
///   `GetCompletion`. They change with the compiler and parser, so the engine only exchanges
///   them with a plugin built from the same Nushell version (see
///   [`ProtocolInfo::same_nushell_version`]). They are outside this version number.
///
/// # When to bump
///
/// | Change to a frozen type | Bump |
/// | --- | --- |
/// | Remove or rename a variant or field, or change what it means | **minor** (`0.1` → `0.2`) |
/// | Add a required field, or change a field's type | **minor** |
/// | Add a variant to an enum inside a message (`Value`, `Category`, `SyntaxShape`, ...) | **minor** |
/// | Add an element to a tuple or newtype variant | **minor** |
/// | Add a named field that is `Option` or `#[serde(default)]` | **patch** |
/// | Add a message that the sending interface only sends after checking the peer's `Hello` | **patch** |
/// | Add or remove a `#[serde(skip)]` field | none |
/// | Documentation only | none |
///
/// Older peers fail to decode an unknown variant or an extra tuple element, and that ends the
/// connection. Only the interfaces in `nu-plugin` and `nu-plugin-engine` see the peer's
/// `Hello`; plugin code that builds a `Signature` or a `Value` doesn't. So the only additions an
/// older peer survives are named fields it skips and messages it is never sent.
///
/// After a bump, add the new messages and variants to the corpus in `src/tests.rs` and run its
/// `regenerate_protocol_snapshots`, which adds `protocol_snapshots/frozen/<new version>/`. The
/// fixtures of every compatible version must keep decoding to the same JSON.
///
/// `protocol_snapshots/README.md` in this crate walks through the tiers, the checks, and the
/// bump procedure step by step, including the plugins that repeat this version.
pub const PLUGIN_PROTOCOL_VERSION: &str = "0.1.0";

/// Protocol information, sent as a `Hello` message on initialization. This determines the
/// compatibility of the plugin and engine. They are considered to be compatible if the lower
/// version is semver compatible with the higher one.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolInfo {
    /// The name of the protocol being implemented. Only one protocol is supported. This field
    /// can be safely ignored, because not matching is a deserialization error
    pub protocol: Protocol,
    /// The semantic version of the plugin wire protocol ([`PLUGIN_PROTOCOL_VERSION`]), not the
    /// Nushell release version.
    pub version: String,
    /// The version of the Nushell crates this side was built from, if it was built from them.
    ///
    /// This does not affect compatibility. It decides whether engine-internal messages (IR
    /// blocks, dynamic completion) can be exchanged; see
    /// [`same_nushell_version`](Self::same_nushell_version). Plugins not written in Rust leave
    /// it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nushell_version: Option<String>,
    /// Supported optional features. This helps to maintain semver compatibility when adding new
    /// features
    pub features: Vec<Feature>,
}

impl Default for ProtocolInfo {
    fn default() -> ProtocolInfo {
        ProtocolInfo {
            protocol: Protocol::NuPlugin,
            version: PLUGIN_PROTOCOL_VERSION.into(),
            nushell_version: Some(env!("CARGO_PKG_VERSION").into()),
            features: default_features(),
        }
    }
}

impl ProtocolInfo {
    /// True if the version specified in `self` is compatible with the version specified in `other`.
    pub fn is_compatible_with(&self, other: &ProtocolInfo) -> Result<bool, ShellError> {
        fn parse_failed(error: semver::Error) -> ShellError {
            ShellError::PluginFailedToLoad {
                msg: format!("Failed to parse protocol version: {error}"),
            }
        }
        let mut versions = [
            semver::Version::parse(&self.version).map_err(parse_failed)?,
            semver::Version::parse(&other.version).map_err(parse_failed)?,
        ];

        versions.sort();

        // The version may carry a prerelease label (e.g. 0.1.0-nightly.1). Clear it so
        // prerelease builds stay compatible with the matching base protocol version.
        versions[1].pre = Prerelease::EMPTY;
        versions[0].pre = Prerelease::EMPTY;

        // For example, if the lower version is 0.1.0, and the higher version is 0.1.3, the
        // requirement is that 0.1.3 matches ^0.1.0 (which it does). On 0.x, 0.2.0 does not.
        Ok(semver::Comparator {
            op: semver::Op::Caret,
            major: versions[0].major,
            minor: Some(versions[0].minor),
            patch: Some(versions[0].patch),
            pre: versions[0].pre.clone(),
        }
        .matches(&versions[1]))
    }

    /// True if both sides were built from the same Nushell version, which is required before
    /// sending engine-internal types (IR blocks, parsed calls) over the wire.
    ///
    /// A side that did not report a Nushell version never matches.
    pub fn same_nushell_version(&self, other: &ProtocolInfo) -> bool {
        self.nushell_version.is_some() && self.nushell_version == other.nushell_version
    }

    /// True if the protocol info contains a feature compatible with the given feature.
    pub fn supports_feature(&self, feature: &Feature) -> bool {
        self.features.iter().any(|f| feature.is_compatible_with(f))
    }
}

/// Indicates the protocol in use. Only one protocol is supported.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum Protocol {
    /// Serializes to the value `"nu-plugin"`
    #[serde(rename = "nu-plugin")]
    #[default]
    NuPlugin,
}

/// Indicates optional protocol features. This can help to make non-breaking-change additions to
/// the protocol. Features are not restricted to plain strings and can contain additional
/// configuration data.
///
/// Optional features should not be used by the protocol if they are not present in the
/// [`ProtocolInfo`] sent by the other side.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "name")]
pub enum Feature {
    /// The plugin supports running with a local socket passed via `--local-socket` instead of
    /// stdio.
    LocalSocket,

    /// A feature that was not recognized on deserialization. Attempting to serialize this feature
    /// is an error. Matching against it may only be used if necessary to determine whether
    /// unsupported features are present.
    #[serde(other, skip_serializing)]
    Unknown,
}

impl Feature {
    /// True if the feature is considered to be compatible with another feature.
    pub fn is_compatible_with(&self, other: &Feature) -> bool {
        matches!((self, other), (Feature::LocalSocket, Feature::LocalSocket))
    }
}

/// Protocol features compiled into this version of `nu-plugin`.
pub fn default_features() -> Vec<Feature> {
    vec![
        // Only available if compiled with the `local-socket` feature flag (enabled by default).
        #[cfg(feature = "local-socket")]
        Feature::LocalSocket,
    ]
}
