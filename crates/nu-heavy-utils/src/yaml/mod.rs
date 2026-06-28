//! YAML parsing and serialization for Nushell values.
//!
//! This module converts between YAML strings and [`nu_protocol::Value`]s through
//! [`parse`] and [`serialize`].
//!
//! The implementation supports YAML 1.1 and YAML 1.2 behavior through [`Spec`].
//! YAML 1.2 is the default, while YAML 1.1 is available for compatibility with
//! older configuration files and YAML type resolution rules.
//!
//! In addition to standard YAML types, the parser and serializer understand
//! Nushell-specific tagged values for variants that YAML cannot represent
//! directly, such as [`CellPath`](nu_protocol::Value::CellPath),
//! [`Duration`](nu_protocol::Value::Duration), [`Filesize`](nu_protocol::Value::Filesize),
//! [`Glob`](nu_protocol::Value::Glob), and [`Range`](nu_protocol::Value::Range).
//! It also recognizes the YAML 1.1
//! [language-independent types](https://yaml.org/type/).
//!
//! Use [`ParseOptions`] and [`SerializeOptions`] to configure spec version,
//! multi-document streams, tag handling, directives, indentation, and related
//! parsing or serialization behavior.
//!
//! ```rust
//! # use nu_heavy_utils::yaml::*;
//! # use nu_protocol::{Span, Spanned, Value};
//! #
//! # let value_span = Span::test_data();
//! # let op_span = Span::test_data();
//! let input = Spanned {
//!     item: "name: nushell\nactive: true",
//!     span: value_span,
//! };
//!
//! let value = parse(input, op_span, ParseOptions::default())?;
//! let yaml = serialize(&value, op_span, SerializeOptions::default())?;
//!
//! assert!(matches!(value, Value::Record { .. }));
//! assert!(yaml.contains("name: nushell"));
//! # Ok::<(), nu_protocol::ShellError>(())
//! ```

use nu_protocol::FromValue;

// Future Consideration
//
// This module is very much built around spanned values, and because of that it hides the
// `ParseError` and `SerializeError` types.
// If we ever run into a case where YAML parsing or serialization is not spanned,
// we can think about exposing the error types instead, work with error sources, and then
// apply the spans in some way once the commands reach users.
// But before we get there, we should make sure things stay spanned wherever possible
// instead of throwing spans away too early.

mod error;

mod parse;
pub use parse::*;

mod serialize;
pub use serialize::*;

/// YAML spec version to comply to.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, FromValue, PartialEq, Eq)]
pub enum Spec {
    /// YAML spec v1.1
    ///
    /// This is a more chaotic version for the YAML specification.
    /// But it is more widely used and many configuration files depend on that version, so this is
    /// available to use.
    ///
    /// <https://yaml.org/spec/1.1>
    #[nu_value(rename = "1.1")]
    V1_1,

    /// YAML spec v1.2
    ///
    /// This is the more reasonable and newer specification for YAML.
    /// By default we comply to this version.
    ///
    /// <https://yaml.org/spec/1.2>
    #[default]
    #[nu_value(rename = "1.2")]
    V1_2,
}

/// Known supported YAML tags.
///
/// This includes the YAML 1.1 language-independent types from
/// <https://yaml.org/type/>, plus Nushell-specific extension tags that map to
/// selected [`nu_protocol::Value`] variants.
#[derive(strum::Display, strum::EnumString, Debug, Clone, Copy)]
#[strum(parse_err_ty = UnknownTagError, parse_err_fn = UnknownTagError::new)]
enum KnownTag {
    // YAML known tags
    /// Unordered mapping language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/map.html>
    #[strum(to_string = "!!map", serialize = "tag:yaml.org,2002:map")]
    Map,

    /// Sequence language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/seq.html>
    #[strum(to_string = "!!seq", serialize = "tag:yaml.org,2002:seq")]
    Seq,

    /// String language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/str.html>
    #[strum(to_string = "!!str", serialize = "tag:yaml.org,2002:str")]
    Str,

    /// Null language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/null.html>
    #[strum(to_string = "!!null", serialize = "tag:yaml.org,2002:null")]
    Null,

    /// Boolean language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/bool.html>
    #[strum(to_string = "!!bool", serialize = "tag:yaml.org,2002:bool")]
    Bool,

    /// Integer language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/int.html>
    #[strum(to_string = "!!int", serialize = "tag:yaml.org,2002:int")]
    Int,

    /// Floating-point language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/float.html>
    #[strum(to_string = "!!float", serialize = "tag:yaml.org,2002:float")]
    Float,

    /// Binary data language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/binary.html>
    #[strum(to_string = "!!binary", serialize = "tag:yaml.org,2002:binary")]
    Binary,

    /// Ordered mapping language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/omap.html>
    #[strum(to_string = "!!omap", serialize = "tag:yaml.org,2002:omap")]
    OMap,

    /// Pairs language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/pairs.html>
    #[strum(to_string = "!!pairs", serialize = "tag:yaml.org,2002:pairs")]
    Pairs,

    /// Set language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/set.html>
    #[strum(to_string = "!!set", serialize = "tag:yaml.org,2002:set")]
    Set,

    /// Merge key language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/merge.html>
    #[strum(to_string = "!!merge", serialize = "tag:yaml.org,2002:merge")]
    Merge,

    /// Timestamp language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/timestamp.html>
    #[strum(to_string = "!!timestamp", serialize = "tag:yaml.org,2002:timestamp")]
    Timestamp,

    /// Value key language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/value.html>
    #[strum(to_string = "!!value", serialize = "tag:yaml.org,2002:value")]
    Value, // not really relevant, just for completeness

    /// YAML encoding keys language-independent type for YAML, version 1.1
    ///
    /// Reference: <https://yaml.org/type/yaml.html>
    #[strum(to_string = "!!yaml", serialize = "tag:yaml.org,2002:yaml")]
    Yaml,

    // Nushell custom tags
    /// Nushell glob value.
    ///
    /// Reference: [`nu_protocol::Value::Glob`]
    #[strum(to_string = "!glob", serialize = "tag:nushell.sh,2026:glob")]
    Glob,

    /// Nushell filesize value.
    ///
    /// Reference: [`nu_protocol::Value::Filesize`]
    #[strum(to_string = "!filesize", serialize = "tag:nushell.sh,2026:filesize")]
    Filesize,

    /// Nushell duration value.
    ///
    /// Reference: [`nu_protocol::Value::Duration`]
    #[strum(to_string = "!duration", serialize = "tag:nushell.sh,2026:duration")]
    Duration,

    /// Nushell range value.
    ///
    /// Reference: [`nu_protocol::Value::Range`]
    #[strum(to_string = "!range", serialize = "tag:nushell.sh,2026:range")]
    Range,

    /// Nushell closure value.
    ///
    /// Reference: [`nu_protocol::Value::Closure`]
    #[strum(to_string = "!closure", serialize = "tag:nushell.sh,2026:closure")]
    Closure,

    /// Nushell error value.
    ///
    /// Reference: [`nu_protocol::Value::Error`]
    #[strum(to_string = "!error", serialize = "tag:nushell.sh,2026:error")]
    Error,

    /// Nushell cell path value.
    ///
    /// Reference: [`nu_protocol::Value::CellPath`]
    #[strum(to_string = "!cell-path", serialize = "tag:nushell.sh,2026:cell-path")]
    CellPath,
}

impl KnownTag {
    /// Nushell custom tag prefix.
    pub const NUSHELL_PREFIX: &str = "tag:nushell.sh,2026:";
}

/// Error if YAML tag is unknown.
struct UnknownTagError(String);

impl UnknownTagError {
    fn new(tag: impl ToString) -> Self {
        Self(tag.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::*;
    use nu_protocol::{engine::Closure, *};
    use nu_test_support::{prelude::*, test_cell_path};
    use rstest::*;
    use std::ops::Bound;

    const SPAN: Span = Span::test_data();

    #[rstest]
    #[case::bool(true)]
    #[case::int(42)]
    #[case::float(1.5)]
    #[case::string("abc")]
    #[case::glob(Value::test_glob("*.nu"))]
    #[case::filesize(Value::test_filesize(1024))]
    #[case::duration(std::time::Duration::from_secs(1))]
    #[case::date(DateTime::UNIX_EPOCH.fixed_offset())]
    #[case::range(Range::new_int(Some(1), Some(2), Some(Bound::Included(4)),))]
    #[case::record(test_record! { "name" => "nushell", "active" => true })]
    #[case::list(test_list!["nushell", 42, true])]
    #[case::binary(Value::test_binary([0, 1, 2, 3]))]
    #[case::cell_path(test_cell_path!(items.0.name))]
    #[case::nothing(())]
    fn roundtrip(#[case] input: impl IntoValue) -> Result {
        let value = input.into_value(SPAN);
        let serialize_options = SerializeOptions::default();
        let yaml = serialize(&value, SPAN, serialize_options)?;
        let parse_options = ParseOptions::default();
        let parsed = parse(yaml.as_str().into_spanned(SPAN), SPAN, parse_options)?;
        assert_eq!(value, parsed);
        Ok(())
    }

    #[rstest]
    #[case::error(ShellError::NushellFailed { msg: "test failure".into() })]
    #[case::closure(Closure { block_id: BlockId::ZERO, captures: vec![] })]
    fn non_roundtrip(#[case] input: impl IntoValue) -> Result {
        let value = input.into_value(SPAN);
        let serialize_options = SerializeOptions::default().with_non_roundtrip(NonRoundtrip::Null);
        let yaml = serialize(&value, SPAN, serialize_options)?;
        let parse_options = ParseOptions::default();
        let parsed = parse(yaml.as_str().into_spanned(SPAN), SPAN, parse_options)?;
        assert!(parsed.is_nothing());
        Ok(())
    }
}
