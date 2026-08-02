mod from;
mod nu_xml_format;
mod to;
mod toml_utils;

// Metadata key used for KDL canonical round-trip mode.
//
// What:
// - `from kdl` writes this key into `PipelineMetadata.custom`.
//
// How:
// - `to kdl` checks this key to decide whether values should be interpreted as
//   canonical node rows (`name/args/props/children`) instead of ordinary
//   Nushell records.
//
// Why:
// - Shape-only detection is ambiguous because user data can legitimately contain
//   the same field names; this explicit marker avoids misclassification.
pub(crate) const KDL_CANONICAL_METADATA_KEY: &str = "nu_kdl_canonical";

// Versioned metadata value paired with `KDL_CANONICAL_METADATA_KEY`.
//
// What:
// - Identifies the canonical KDL node-row schema variant.
//
// How:
// - `from kdl` writes this value.
// - `to kdl` only enables canonical handling when both key and value match.
//
// Why:
// - Versioning keeps the contract forward-compatible if the canonical schema
//   evolves in future changes.
pub(crate) const KDL_CANONICAL_METADATA_VALUE: &str = "node_rows_v1";

pub use from::*;
pub use to::*;

pub(crate) use toml_utils::{preserve_toml_document, read_toml_source_from_metadata};
