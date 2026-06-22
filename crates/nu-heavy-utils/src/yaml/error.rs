use std::num::{ParseFloatError, ParseIntError};

use nu_protocol::{
    ParseRangeError, ShellError, Span, ast::ParseCellPathError, shell_error::generic::GenericError,
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
    NumInt {
        base: u32,
        attempted: String,
        err: ParseIntError,
        span: Span,
    },
    NumFloat {
        attempted: String,
        err: ParseFloatError,
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
    Document,
    Scalar,
    Sequence,
    Mapping,
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
            .with_code("shell::yaml::parser::too_many_documents"),

            ParseError::Scan { source, span } => GenericError::new(
                "Scanning YAML failed",
                "Scanning the YAML input failed",
                span,
            )
            .with_code("shell::yaml::parser::scan")
            .with_source(source),

            ParseError::NumInt {
                base,
                attempted,
                err,
                span,
            } => GenericError::new(
                format!("Parsing Base {base} failed"),
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code(format!("shell::yaml::parser::num::base{base}"))
            .with_source(err),

            ParseError::NumFloat {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Float failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parser::num::float")
            .with_source(err),

            ParseError::Bool { attempted, span } => GenericError::new(
                "Parsing Bool failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parser::bool"),

            ParseError::Binary {
                attempted,
                err,
                span,
            } => GenericError::new(
                "Parsing Binary failed",
                format!("Parsing {attempted:?} failed"),
                span,
            )
            .with_code("shell::yaml::parser::binary")
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
            .with_code("shell::yaml::parser::date")
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
            .with_code("shell::yaml::parser::range")
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
            .with_code("shell::yaml::parser::cell_path")
            .with_source(err),

            ParseError::UnknownTag { tag, span } => GenericError::new(
                "Unknown tag",
                format!("The tag {:?} is unknown to nushell", tag),
                span,
            )
            .with_code("shell::yaml::parser::tag::unknown"),

            ParseError::UnimplementedTag { tag, span } => GenericError::new(
                "Unimplemented Tag",
                format!("The tag {tag} is known but not implemented"),
                span,
            )
            .with_code("shell::yaml::parser::tag::unimplemented"),

            ParseError::IncorrectTag { tag, at, span } => GenericError::new(
                "Incorrect tag",
                format!("Found incorrect tag {tag} while parsing {at}"),
                span,
            )
            .with_code("shell::yaml::parser::tag::incorrect"),

            ParseError::UnsupportedTag { tag, span } => GenericError::new(
                "Unsupported tag",
                format!("The tag {tag} is generally not supported"),
                span,
            )
            .with_code("shell::yaml::parser::tag::unsupported"),

            ParseError::Internal { error, span } => GenericError::new(
                "Internal YAML Parser Error",
                "The YAML parser got into an unexpected state",
                span,
            )
            .with_code("shell::yaml::parser::internal")
            .with_help("This is most likely a bug. Please report it.")
            .with_inner([ShellError::Generic(match error {
                InternalParserError::UnexpectedEvent { event, location } => {
                    GenericError::new_internal_with_location(
                        "Unexpected YAML event",
                        format!("Unexpected YAML event during parsing: {event:?}"),
                        location,
                    )
                    .with_code("shell::yaml::parser::internal::unexpected_event")
                }

                InternalParserError::UnexpectedEventEnd { location } => {
                    GenericError::new_internal_with_location(
                        "Unexpected end of YAML events",
                        "Unexpectedly the event stream of the YAML parser ended",
                        location,
                    )
                    .with_code("shell::yaml::parser::internal::end_of_events")
                }

                InternalParserError::ZeroAliasID { location } => {
                    GenericError::new_internal_with_location(
                        "Invalid Alias ID",
                        "YAML parser generated 0 as an Alias ID",
                        location,
                    )
                    .with_code("shell::yaml::parser::internal::zero_alias")
                }

                InternalParserError::MissingAlias { location } => {
                    GenericError::new_internal_with_location(
                        "Missing Alias",
                        "Could not find value for Alias",
                        location,
                    )
                    .with_code("shell::yaml::parser::internal::missing_alias")
                }
            })]),
        };

        ShellError::Generic(error)
    }
}

pub enum SerializeError {}
