use std::num::{ParseFloatError, ParseIntError};

use nu_protocol::{
    ParseRangeError, ShellError, Span, Type, ast::ParseCellPathError,
    shell_error::generic::GenericError,
};
use nu_utils::location::Location;
use serde_saphyr::granit_parser::{Event, ScanError};

use crate::yaml::KnownTag;

#[expect(
    private_interfaces,
    reason = "KnownTag has the same visibility as this error"
)]
pub enum ParseError<'i> {
    TooManyDocuments {
        span: Span,
    },
    Scan {
        source: ScanError,
        span: Span,
    },
    DuplicateKey {
        duplicate: String,
        span: Span,
    },
    UnexpectedKeyAnchor {
        span: Span,
    },
    Int {
        attempted: String,
        base_and_err: Option<(u32, ParseIntError)>,
        span: Span,
    },
    Float {
        attempted: String,
        base_and_err: Option<(u32, ParseFloatError)>,
        span: Span,
    },
    Null {
        attempted: String,
        span: Span,
    },
    Bool {
        attempted: String,
        span: Span,
    },
    Binary {
        attempted: String,
        err: base64::DecodeError,
        span: Span,
    },
    Date {
        attempted: String,
        err: chrono::ParseError,
        span: Span,
    },
    Range {
        attempted: String,
        err: ParseRangeError,
        span: Span,
    },
    CellPath {
        attempted: String,
        err: ParseCellPathError,
        span: Span,
    },
    PairsNotARecord {
        found: Type,
        span: Span,
    },
    PairsEmpty {
        span: Span,
    },
    PairsTooMany {
        span: Span,
    },
    OMapNotARecord {
        found: Type,
        span: Span,
    },
    OMapEmpty {
        span: Span,
    },
    OMapDuplicateKey {
        duplicate: String,
        span: Span,
    },
    OMapTooMany {
        span: Span,
    },
    SetFoundNotNull {
        found: Type,
        span: Span,
    },
    UnimplementedTag {
        tag: KnownTag,
        span: Span,
    },
    UnknownTag {
        tag: String,
        span: Span,
    },
    IncorrectTag {
        tag: KnownTag,
        at: NodeKind,
        span: Span,
    },
    UnsupportedTag {
        tag: KnownTag,
        at: NodeKind,
        span: Span,
    },
    InvalidMergeType {
        found: Type,
        span: Span,
    },
    InvalidMergeList {
        found: Type,
        span: Span,
    },
    Internal {
        error: InternalParserError<'i>,
        span: Span,
    },
}

#[derive(strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum NodeKind {
    Scalar,
    Sequence,
    Mapping,
    Key,
}

pub enum InternalParserError<'i> {
    UnexpectedEvent {
        event: Event<'i>,
        location: Location,
    },
    UnexpectedEventEnd {
        location: Location,
    },
    ZeroAliasID {
        location: Location,
    },
    MissingAlias {
        location: Location,
    },
}

impl From<ParseError<'_>> for ShellError {
    #[track_caller]
    fn from(error: ParseError) -> Self {
        let error = match error {
            ParseError::TooManyDocuments { span } => GenericError::new(
                "Too many documents",
                "Found more than one document, but requested only one",
                span,
            )
            .with_code("shell::yaml::parse::too_many_documents"),

            ParseError::Scan { source, span } => GenericError::new(
                "Scanning YAML failed",
                "Scanning the YAML input failed",
                span,
            )
            .with_code("shell::yaml::parse::scan")
            .with_source(source),

            ParseError::DuplicateKey { duplicate, span } => GenericError::new(
                "Duplicate Mapping Key",
                format!("The key {duplicate:?} already appeared in the mapping"),
                span,
            )
            .with_code("shell::yaml::parse::duplicate_key"),

            ParseError::UnexpectedKeyAnchor { span } => GenericError::new(
                "Found unexpected key anchor",
                "Merge anchors are not supported in key position",
                span,
            )
            .with_code("shell::yaml::parse::unexpected_key_anchor"),

            ParseError::Int {
                attempted,
                base_and_err: Some((base, err)),
                span,
            } => GenericError::new(
                format!("Parsing Int Base {base} failed"),
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code(format!("shell::yaml::parse::int::base{base}"))
            .with_source(err),

            ParseError::Int {
                attempted,
                base_and_err: None,
                span,
            } => GenericError::new(
                format!("Parsing Int failed"),
                format!("Could not identify {attempted:?} as an int"),
                span,
            )
            .with_code(format!("shell::yaml::parse::int::unknown")),

            ParseError::Float {
                attempted,
                base_and_err: Some((base, err)),
                span,
            } => GenericError::new(
                format!("Parsing Float Base {base} failed"),
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code(format!("shell::yaml::parse::float::base{base}"))
            .with_source(err),

            ParseError::Float {
                attempted,
                base_and_err: None,
                span,
            } => GenericError::new(
                format!("Parsing Float failed"),
                format!("Could not identify {attempted:?} as a float"),
                span,
            )
            .with_code(format!("shell::yaml::parse::float::unknown")),

            ParseError::Null { attempted, span } => GenericError::new(
                "Parsing Null failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::null"),

            ParseError::Bool { attempted, span } => GenericError::new(
                "Parsing Bool failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::bool"),

            ParseError::Binary {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Binary failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::binary")
            .with_source(err),

            ParseError::Date {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Date failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::date")
            .with_source(err),

            ParseError::Range {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Range failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::range")
            .with_source(err),

            ParseError::CellPath {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Cell Path failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parse::cell_path")
            .with_source(err),

            ParseError::PairsNotARecord { found, span } => GenericError::new(
                "Pairs has to be a record",
                format!("Expected {}, found {}", Type::record(), found),
                span,
            )
            .with_code("shell::yaml::parse::pairs::not_a_record"),

            ParseError::PairsEmpty { span } => GenericError::new(
                "Pairs entry is empty",
                format!(
                    "While handling {} tag, found an empty entry",
                    KnownTag::Pairs
                ),
                span,
            )
            .with_code("shell::yaml::parse::pairs::empty"),

            ParseError::PairsTooMany { span } => GenericError::new(
                "Pairs entry has to many entries",
                format!(
                    "While handling {} tag, found an entry with too many entries",
                    KnownTag::Pairs
                ),
                span,
            )
            .with_code("shell::yaml::parse::pairs::too_many"),

            ParseError::OMapNotARecord { found, span } => GenericError::new(
                "OMap has to be a record",
                format!("Expected {}, found {}", Type::record(), found),
                span,
            )
            .with_code("shell::yaml::parse::omap::not_a_record"),

            ParseError::OMapEmpty { span } => GenericError::new(
                "OMap entry is empty",
                format!(
                    "While handling {} tag, found an empty entry",
                    KnownTag::OMap
                ),
                span,
            )
            .with_code("shell::yaml::parse::omap::empty"),

            ParseError::OMapDuplicateKey { duplicate, span } => GenericError::new(
                "Duplicate OMap key found",
                format!("Found duplicate key {duplicate:?}, OMap does not support duplicate keys"),
                span,
            )
            .with_code("shell::yaml::parse::omap::duplicate_key"),

            ParseError::OMapTooMany { span } => GenericError::new(
                "OMap entry has to many entries",
                format!(
                    r#"While handling "{}" tag, found an entry with too many entries"#,
                    KnownTag::OMap
                ),
                span,
            )
            .with_code("shell::yaml::parse::omap::too_many"),

            ParseError::SetFoundNotNull { found, span } => GenericError::new(
                "Found not null in Set",
                format!(
                    r#"While handling "{}", expected values only to be {}, found {found}"#,
                    KnownTag::Set,
                    Type::Nothing
                ),
                span,
            )
            .with_code("shell::yaml::parse::set::not_a_null"),

            ParseError::UnknownTag { tag, span } => GenericError::new(
                "Unknown tag",
                format!("The tag {:?} is unknown to nushell", tag),
                span,
            )
            .with_code("shell::yaml::parse::tag::unknown"),

            ParseError::UnimplementedTag { tag, span } => GenericError::new(
                "Unimplemented Tag",
                format!(r#"The tag "{tag}" is known but not implemented"#),
                span,
            )
            .with_code("shell::yaml::parse::tag::unimplemented"),

            ParseError::IncorrectTag { tag, at, span } => GenericError::new(
                "Incorrect tag",
                format!(r#"Found incorrect tag "{tag}" while parsing a {at}"#),
                span,
            )
            .with_code("shell::yaml::parse::tag::incorrect"),

            ParseError::UnsupportedTag { tag, at, span } => GenericError::new(
                "Unsupported tag",
                format!(r#"The tag "{tag}" is not supported while parsing a {at}"#),
                span,
            )
            .with_code("shell::yaml::parse::tag::unsupported"),

            ParseError::InvalidMergeType { found, span } => GenericError::new(
                "Invalid merge type",
                format!(
                    "Expected {} or {}, found {found}",
                    Type::record(),
                    Type::list(Type::record())
                ),
                span,
            )
            .with_code("shell::yaml::parse::merge::invalid_type"),

            ParseError::InvalidMergeList { found, span } => GenericError::new(
                "Invalid merge list type",
                format!("Expected {} inside the list, found {found}", Type::record()),
                span,
            )
            .with_code("shell::yaml::parse::merge::invalid_list_type"),

            ParseError::Internal { error, span } => GenericError::new(
                "Internal YAML Parser Error",
                "The YAML parser got into an unexpected state",
                span,
            )
            .with_code("shell::yaml::parse::internal")
            .with_help("This is most likely a bug. Please report it.")
            .with_inner([ShellError::Generic(match error {
                InternalParserError::UnexpectedEvent { event, location } => {
                    GenericError::new_internal_with_location(
                        "Unexpected YAML event",
                        format!("Unexpected YAML event during parsing: {event:?}"),
                        location,
                    )
                    .with_code("shell::yaml::parse::internal::unexpected_event")
                }

                InternalParserError::UnexpectedEventEnd { location } => {
                    GenericError::new_internal_with_location(
                        "Unexpected end of YAML events",
                        "Unexpectedly the event stream of the YAML parser ended",
                        location,
                    )
                    .with_code("shell::yaml::parse::internal::end_of_events")
                }

                InternalParserError::ZeroAliasID { location } => {
                    GenericError::new_internal_with_location(
                        "Invalid Alias ID",
                        "YAML parser generated 0 as an Alias ID",
                        location,
                    )
                    .with_code("shell::yaml::parse::internal::zero_alias")
                }

                InternalParserError::MissingAlias { location } => {
                    GenericError::new_internal_with_location(
                        "Missing Alias",
                        "Could not find value for Alias",
                        location,
                    )
                    .with_code("shell::yaml::parse::internal::missing_alias")
                }
            })]),
        };

        ShellError::Generic(error)
    }
}

pub enum SerializeError {
    Serializer {
        err: serde_saphyr::ser::Error,
        span: Span,
    },
    UnsupportedCustomValue {
        type_name: String,
        span: Span,
    },
}

impl From<SerializeError> for ShellError {
    fn from(value: SerializeError) -> Self {
        let (err, span) = match value {
            SerializeError::Serializer { err, span } => (err, span),
            SerializeError::UnsupportedCustomValue { type_name, span } => {
                return ShellError::Generic(
                    GenericError::new(
                        "Unsupported custom values",
                        format!("Cannot convert custom value `{type_name}` into YAML"),
                        span,
                    )
                    .with_code("shell::yaml::serialize::unsupported_custom_value")
                    .with_help("Try to call `into value` on the custom value first"),
                );
            }
        };

        use serde_saphyr::ser::Error as SerError;
        ShellError::Generic(match err {
            SerError::Message { msg } if msg == SerializeError::CLOSURE_SPAN_NOT_FOUND => {
                GenericError::new(
                    "Closure span not found",
                    "Could not find the span of the closure to serialize it",
                    span,
                )
                .with_code("shell::yaml::serialize::closure_span_not_found")
            }
            SerError::Message { msg } => GenericError::new("Serialization failed", msg, span)
                .with_code("shell::yaml::serialize"),
            SerError::Format { error } => {
                GenericError::new("Format error during serialization", error.to_string(), span)
                    .with_code("shell::yaml::serialize::fmt")
            }
            SerError::IO { .. } => unreachable!("we only serialize to a string, so no IO"),
            SerError::Unexpected { msg } => {
                GenericError::new("Unexpected serialization error", msg, span)
                    .with_code("shell::yaml::serialize::unexpected")
            }
            SerError::InvalidOptions(msg) => GenericError::new("Invalid options", msg, span)
                .with_code("shell::yaml::serialize::invalid_options"),
            SerError::SingleQuotedRequiresEscaping { ch } => GenericError::new(
                "Single quoted requires escaping",
                format!("{ch:?} requires escaping"),
                span,
            )
            .with_code("shell::yaml::serialize::requires_escaping"),
            err => GenericError::new("Serialization failed", err.to_string(), span)
                .with_code("shell::yaml::serialize"),
        })
    }
}

impl SerializeError {
    pub const CLOSURE_SPAN_NOT_FOUND: &str = concat!(module_path!(), "::closure_span_not_found");
}
