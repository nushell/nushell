//! YAML parsing.
//!
//! This module implements the YAML parser at a low level by using
//! [`granit_parser`](serde_saphyr::granit_parser) instead of the higher-level
//! [`serde_saphyr`] data types.
//!
//! This lets us be a lot more precise and makes it easier to differentiate between
//! YAML 1.1 and 1.2 spec compliance.
//!
//! Only the high-level parsing API should be exposed.
//! Internal changes should not be considered public API.
//!
//! For parsing scalar values, we use the regexes from the
//! [types of the 1.1 spec](https://yaml.org/type/).
//! These usually allow a lot more complex formats than 1.2, so they need to be split
//! carefully.
//!
//! For 1.2 scalars, we refer to the
//! [core schema of YAML](https://yaml.org/spec/1.2.2/#103-core-schema).
//!
//! Throughout these functions, we refer to two different spans:
//!
//! - `yaml_span` points to the span in the input string.
//!   Errors caused by bad YAML in any way should use this span.
//! - `parser_span` points to the parser or implementation span.
//!   Errors caused by the parser or by our implementation, but while handling
//!   otherwise correct YAML, should use this span instead.
//!   For example, this applies when we don't support a certain syntax yet.
//!
//! This documentation is private to the implementors, as this module itself is not
//! public.
//! Only [`parse`], [`ParseOptions`], and their field types are public.

use crate::{
    merge::{Merge, MergeStrategy},
    yaml::{
        KnownTag, Spec, UnknownTagError,
        error::{InternalParserError, NodeKind, ParseError, TimestampIssue},
    },
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::DateTime;
use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use derive_setters::Setters;
use nu_protocol::{
    FromValue, Range, Record, ShellError, Span, Spanned, Type, Value, ast::CellPath, record,
};
use nu_utils::location::Location;
use regex::{Captures, Regex};
use serde_saphyr::granit_parser::{Event, Parser, ScalarStyle, StrInput, StructureStyle, Tag};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};

/// Options for parsing YAML.
///
/// Use this to configure how the parser works.
///
/// This type provides builder-style setters directly, so options can be chained while building it.
///
/// ```rust
/// # use nu_heavy_utils::yaml::*;
/// #
/// let options = ParseOptions::default()
///     .with_multiple(ParseMultiple::ForceList)
///     .with_spec(Spec::V1_1)
///     .with_ignore_tags(true);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
#[setters(prefix = "with_")]
pub struct ParseOptions {
    /// Keep the original styles of parsed values.
    ///
    /// This allows serializing them later in the same style they were written in.
    #[deprecated = "not implemented yet"]
    pub keep_styles: bool,

    /// Configure how multiple documents in a YAML stream are handled.
    pub multiple: ParseMultiple,

    /// Configure which YAML spec to follow.
    pub spec: Spec,

    /// Ignore any tags found during parsing.
    ///
    /// This also ignores Nushell's custom tags.
    /// Type hints from those tags are removed, so this returns basic types instead of more complex
    /// variants like [`Value::CellPath`].
    pub ignore_tags: bool,

    /// Configure how plain scalar keys are handled.
    pub key_resolution: KeyResolution,
}

/// Configure how multiple documents in a YAML stream are handled.
#[derive(Debug, Clone, Copy, Default, FromValue)]
pub enum ParseMultiple {
    /// Use the default automatic behavior.
    ///
    /// A single document is returned directly.
    /// If the YAML input contains multiple documents, they are returned as a list.
    #[default]
    #[nu_value(rename = "auto")]
    Auto,

    /// Always return a list.
    ///
    /// Even a single document is wrapped in a list.
    #[nu_value(rename = "list")]
    ForceList,

    /// Always return a single document.
    ///
    /// If multiple documents are found, an error is returned.
    #[nu_value(rename = "single")]
    ForceSingle,
}

/// How plain scalar mapping keys are handled when they resolve to non-string values.
///
/// YAML plain scalars can resolve to values other than strings, such as `null`, booleans,
/// or numbers.
/// A [`Record`] cannot represent those values as mapping keys.
///
/// By default, such keys are rejected.
/// For compatibility with looser YAML usage, [`KeyResolution::Verbatim`] can instead keep the
/// original plain scalar text as the key.
#[derive(Debug, Clone, Copy, Default, FromValue, PartialEq, Eq)]
#[nu_value(rename_all = "snake_case")]
pub enum KeyResolution {
    /// Reject plain scalar keys that resolve to non-string values.
    #[default]
    Strict,

    /// Use the plain scalar source text as the key when it would resolve to a non-string value.
    ///
    /// This is more compatible with loose YAML usage, but does not fully preserve YAML's data model.
    Verbatim,
}

/// Parse a YAML string into a [`Value`].
///
/// See [`ParseOptions`] for behavior that can change how YAML is interpreted.
/// `yaml` provides the input span for source errors, `span` is used for parser
/// errors and parsed values.
pub fn parse(yaml: Spanned<&str>, span: Span, options: ParseOptions) -> Result<Value, ShellError> {
    let parser = Parser::new_from_str(yaml.item);
    let ctx = &mut ParseCtx {
        parser,
        parser_span: span,
        yaml_span: yaml.span,
        anchors: HashMap::new(),
        options: &options,
    };

    let start = ctx.next_event()?;
    match start {
        Event::StreamStart => (),
        event => return Err(ctx.unexpected_event(event).into()),
    }

    let mut documents = Vec::new();
    loop {
        match ctx.next_event()? {
            Event::DocumentStart(_) => documents.push(parse_document(ctx)?),
            Event::StreamEnd => break,
            Event::Nothing | Event::Comment(..) => continue,
            event => return Err(ctx.unexpected_event(event).into()),
        }
    }

    use ParseMultiple as PM;
    let value = match (ctx.options.multiple, documents.len()) {
        (PM::Auto | PM::ForceSingle, 0) => Value::nothing(ctx.parser_span),
        (PM::Auto | PM::ForceSingle, 1) => documents.into_iter().next().expect("non-empty"),
        (PM::Auto | PM::ForceList, _) => Value::list(documents, ctx.parser_span),
        (PM::ForceSingle, _) => {
            return Err(ShellError::from(ParseError::TooManyDocuments {
                span: ctx.yaml_span,
            }));
        }
    };

    Ok(value)
}

struct ParseCtx<'i> {
    parser: Parser<'i, StrInput<'i>>,
    parser_span: Span,
    yaml_span: Span,
    anchors: HashMap<NonZeroUsize, Value>,
    options: &'i ParseOptions,
}

impl<'i> ParseCtx<'i> {
    #[track_caller]
    fn next_event(&mut self) -> Result<Event<'i>, ParseError<'i>> {
        match self.parser.next_event() {
            None => Err(ParseError::Internal {
                error: InternalParserError::UnexpectedEventEnd {
                    location: Location::caller(),
                },
                span: self.parser_span,
            }),
            Some(Err(err)) => Err(ParseError::Scan {
                source: err,
                span: self.yaml_span,
            }),
            Some(Ok((event, _))) => Ok(event),
        }
    }

    #[track_caller]
    fn unexpected_event(&self, event: Event<'i>) -> ParseError<'i> {
        ParseError::Internal {
            error: InternalParserError::UnexpectedEvent {
                event: event,
                location: Location::caller(),
            },
            span: self.parser_span,
        }
    }

    fn unknown_tag_err(&self, err: UnknownTagError) -> ParseError<'i> {
        ParseError::UnknownTag {
            tag: err.0,
            span: self.yaml_span,
        }
    }

    #[track_caller]
    fn alias(&self, id: usize) -> Result<NonZeroUsize, ParseError<'i>> {
        NonZeroUsize::new(id).ok_or(ParseError::Internal {
            error: InternalParserError::ZeroAliasID {
                location: Location::caller(),
            },
            span: self.parser_span,
        })
    }

    fn set_anchor(&mut self, anchor_id: NonZeroUsize, value: Value) {
        self.anchors.insert(anchor_id, value);
    }

    fn maybe_set_anchor(&mut self, anchor_id: usize, value: &Value) {
        NonZeroUsize::new(anchor_id).map(|anchor_id| self.set_anchor(anchor_id, value.clone()));
    }

    #[track_caller]
    fn get_anchor(&self, anchor_id: NonZeroUsize) -> Result<Value, ShellError> {
        match self.anchors.get(&anchor_id) {
            Some(value) => Ok(value.clone()),
            None => Err(ShellError::from(ParseError::Internal {
                error: InternalParserError::MissingAlias {
                    location: Location::caller(),
                },
                span: self.parser_span,
            })),
        }
    }

    fn resolve_tag(&self, tag: Option<&Tag>) -> Result<Option<KnownTag>, ParseError<'i>> {
        if self.options.ignore_tags {
            return Ok(None);
        }

        tag.map(|tag| KnownTag::from_str(tag.to_string().as_str()))
            .transpose()
            .map_err(|err| self.unknown_tag_err(err))
    }
}

fn parse_document<'i>(ctx: &mut ParseCtx<'i>) -> Result<Value, ShellError> {
    // TODO: if document version directive gets exposed, read it and override the spec version locally

    let value = loop {
        match ctx.next_event()? {
            Event::Nothing | Event::Comment(..) => continue,
            Event::Alias(anchor_id) => break ctx.get_anchor(ctx.alias(anchor_id)?)?,
            Event::Scalar(value, scalar_style, anchor_id, tag) => {
                let value = parse_scalar(ctx, value, scalar_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                break value;
            }
            Event::SequenceStart(structure_style, anchor_id, tag) => {
                let value = parse_sequence(ctx, structure_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                break value;
            }
            Event::MappingStart(structure_style, anchor_id, tag) => {
                let value = parse_mapping(ctx, structure_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                break value;
            }
            event => return Err(ctx.unexpected_event(event).into()),
        }
    };

    loop {
        match ctx.next_event()? {
            Event::Nothing | Event::Comment(..) => continue,
            Event::DocumentEnd => return Ok(value),
            event => return Err(ctx.unexpected_event(event).into()),
        }
    }
}

fn parse_scalar<'i>(
    ctx: &mut ParseCtx<'i>,
    value: Cow<'i, str>,
    scalar_style: ScalarStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    let tag = ctx.resolve_tag(tag.as_deref())?;
    let value = value.as_ref();

    match tag {
        None => parse_scalar_untagged(ctx, value, scalar_style),
        Some(tag) => parse_scalar_tagged(ctx, value, tag),
    }
}

fn parse_scalar_untagged<'i>(
    ctx: &mut ParseCtx<'i>,
    value: &str,
    scalar_style: ScalarStyle,
) -> Result<Value, ShellError> {
    let span = ctx.parser_span;

    match scalar_style {
        ScalarStyle::Plain => (),

        // without tags, these can only be strings.
        ScalarStyle::SingleQuoted
        | ScalarStyle::DoubleQuoted
        | ScalarStyle::Literal
        | ScalarStyle::Folded => return Ok(Value::string(value, span)),
    }

    use Spec::{V1_1, V1_2};
    #[deny(non_snake_case, reason = "ensure we don't suddenly wildcard match")]
    Ok(match (ctx.options.spec, value) {
        (_, s) if let Some(()) = v1_x::maybe_parse_null(ctx, s) => Value::nothing(span),
        (V1_1, s) if let Some(b) = v1_1::maybe_parse_bool(ctx, s) => Value::bool(b, span),
        (V1_2, s) if let Some(b) = v1_2::maybe_parse_bool(ctx, s) => Value::bool(b, span),
        (V1_1, s) if let Some(i) = v1_1::maybe_parse_int(ctx, s)? => Value::int(i, span),
        (V1_2, s) if let Some(i) = v1_2::maybe_parse_int(ctx, s)? => Value::int(i, span),
        (V1_1, s) if let Some(f) = v1_1::maybe_parse_float(ctx, s)? => Value::float(f, span),
        (V1_2, s) if let Some(f) = v1_2::maybe_parse_float(ctx, s)? => Value::float(f, span),
        (V1_1, s) if let Some(ts) = v1_x::maybe_parse_timestamp(ctx, s)? => Value::date(ts, span),
        (_, s) => Value::string(s, span),
    })
}

fn parse_scalar_tagged<'i>(
    ctx: &mut ParseCtx<'i>,
    value: &str,
    tag: KnownTag,
) -> Result<Value, ShellError> {
    let span = ctx.parser_span;

    use Spec::{V1_1, V1_2};
    #[deny(non_snake_case, reason = "ensure we don't suddenly wildcard match")]
    Ok(match (tag, ctx.options.spec) {
        (KnownTag::Str, _) => Value::string(value, span),
        (KnownTag::Null, _) => v1_x::maybe_parse_null(ctx, value)
            .map(|()| Value::nothing(span))
            .ok_or_else(|| ParseError::Null {
                attempted: value.to_owned(),
                span: ctx.yaml_span,
            })?,
        (KnownTag::Bool, V1_1) => v1_1::maybe_parse_bool(ctx, value)
            .map(|bool| Value::bool(bool, span))
            .ok_or_else(|| ParseError::Bool {
                attempted: value.to_owned(),
                span: ctx.yaml_span,
            })?,
        (KnownTag::Bool, V1_2) => v1_2::maybe_parse_bool(ctx, value)
            .map(|bool| Value::bool(bool, span))
            .ok_or_else(|| ParseError::Bool {
                attempted: value.to_owned(),
                span: ctx.yaml_span,
            })?,
        (KnownTag::Int, V1_1) => v1_1::maybe_parse_int(ctx, value)?
            .map(|int| Value::int(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Int, V1_2) => v1_2::maybe_parse_int(ctx, value)?
            .map(|int| Value::int(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Float, V1_1) => v1_1::maybe_parse_float(ctx, value)?
            .map(|float| Value::float(float, span))
            .ok_or_else(|| ParseError::Float {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Float, V1_2) => v1_2::maybe_parse_float(ctx, value)?
            .map(|float| Value::float(float, span))
            .ok_or_else(|| ParseError::Float {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Binary, _) => Value::binary(
            BASE64_STANDARD
                .decode(
                    // for the base64 value, we need to filter out any whitespace
                    value
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>(),
                )
                .map_err(|err| ParseError::Binary {
                    attempted: value.to_owned(),
                    err,
                    span: ctx.yaml_span,
                })?,
            span,
        ),
        (KnownTag::Glob, _) => Value::glob(value, false, span),
        (KnownTag::Filesize, V1_1) => v1_1::maybe_parse_int(ctx, value)?
            .map(|int| Value::filesize(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Filesize, V1_2) => v1_2::maybe_parse_int(ctx, value)?
            .map(|int| Value::filesize(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Duration, V1_1) => v1_1::maybe_parse_int(ctx, value)?
            .map(|int| Value::duration(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Duration, V1_2) => v1_2::maybe_parse_int(ctx, value)?
            .map(|int| Value::duration(int, span))
            .ok_or_else(|| ParseError::Int {
                attempted: value.to_owned(),
                base_and_err: None,
                span,
            })?,
        (KnownTag::Timestamp, _) => v1_x::maybe_parse_timestamp(ctx, value)?
            .map(|ts| Value::date(ts, span))
            .ok_or_else(|| ParseError::Timestamp {
                attempted: value.to_owned(),
                issue: None,
                span,
            })?,
        (KnownTag::Range, _) => Value::range(
            Range::from_str(value).map_err(|err| ParseError::Range {
                attempted: value.to_owned(),
                err,
                span: ctx.yaml_span,
            })?,
            span,
        ),
        (KnownTag::CellPath, _) => Value::cell_path(
            CellPath::from_str(value)
                .map(|cp| cp.with_fallback_span(span))
                .map_err(|err| ParseError::CellPath {
                    attempted: value.to_owned(),
                    err,
                    span: ctx.yaml_span,
                })?,
            span,
        ),

        // unimplemented tag
        (KnownTag::Closure | KnownTag::Error, _) => {
            return Err(ShellError::from(ParseError::UnimplementedTag { tag, span }));
        }

        // incorrect tag
        (
            KnownTag::Map
            | KnownTag::Seq
            | KnownTag::OMap
            | KnownTag::Pairs
            | KnownTag::Set
            | KnownTag::Merge,
            _,
        ) => {
            return Err(ShellError::from(ParseError::IncorrectTag {
                tag,
                at: NodeKind::Scalar,
                span: ctx.yaml_span,
            }));
        }

        // unsupported tag
        (KnownTag::Value | KnownTag::Yaml, _) => {
            return Err(ShellError::from(ParseError::UnsupportedTag {
                tag,
                at: NodeKind::Scalar,
                span,
            }));
        }
    })
}

fn parse_sequence<'i>(
    ctx: &mut ParseCtx<'i>,
    _structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    let tag = ctx.resolve_tag(tag.as_deref())?;

    let mut values = Vec::new();
    loop {
        match ctx.next_event()? {
            Event::Nothing | Event::Comment(..) => continue,
            Event::Alias(anchor_id) => values.push(ctx.get_anchor(ctx.alias(anchor_id)?)?),
            Event::Scalar(value, scalar_style, anchor_id, tag) => {
                let value = parse_scalar(ctx, value, scalar_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                values.push(value);
            }
            Event::SequenceStart(structure_style, anchor_id, tag) => {
                let value = parse_sequence(ctx, structure_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                values.push(value);
            }
            Event::MappingStart(structure_style, anchor_id, tag) => {
                let value = parse_mapping(ctx, structure_style, tag)?;
                ctx.maybe_set_anchor(anchor_id, &value);
                values.push(value);
            }
            Event::SequenceEnd => break,
            event => return Err(ctx.unexpected_event(event).into()),
        }
    }

    let tag = tag.unwrap_or(KnownTag::Seq);
    match tag {
        KnownTag::Seq => Ok(Value::list(values, ctx.parser_span)),
        KnownTag::Pairs => {
            let mut pairs = Vec::with_capacity(values.len());
            for value in values.into_iter() {
                let span = value.span();
                let ty = value.get_type();
                let record = value
                    .into_record()
                    .map_err(|_| ParseError::PairsNotARecord {
                        found: ty,
                        span: ctx.yaml_span,
                    })?;
                let mut iter = record.into_iter();
                pairs.push(match (iter.next(), iter.next()) {
                    (None, None) => Err(ParseError::PairsEmpty {
                        span: ctx.yaml_span,
                    }),
                    (Some((key, value)), None) => Ok(Value::record(
                        record!(
                            "key" => Value::string(key, span),
                            "value" => value
                        ),
                        span,
                    )),
                    (_, Some(_)) => Err(ParseError::PairsTooMany {
                        span: ctx.yaml_span,
                    }),
                }?);
            }
            Ok(Value::list(pairs, ctx.parser_span))
        }
        KnownTag::OMap => {
            let mut entries = Vec::with_capacity(values.len());
            let mut keys = HashSet::with_capacity(values.len());
            for value in values.into_iter() {
                let ty = value.get_type();
                let record = value
                    .into_record()
                    .map_err(|_| ParseError::OMapNotARecord {
                        found: ty,
                        span: ctx.yaml_span,
                    })?;
                let mut iter = record.into_iter();
                match (iter.next(), iter.next()) {
                    (None, None) => {
                        return Err(ShellError::from(ParseError::OMapEmpty {
                            span: ctx.yaml_span,
                        }));
                    }
                    (Some((key, value)), None) => {
                        if !keys.insert(key.clone()) {
                            return Err(ShellError::from(ParseError::OMapDuplicateKey {
                                duplicate: key.clone(),
                                span: ctx.yaml_span,
                            }));
                        }

                        entries.push((key, value))
                    }
                    (_, Some(_)) => {
                        return Err(ShellError::from(ParseError::OMapTooMany {
                            span: ctx.yaml_span,
                        }));
                    }
                }
            }
            Ok(Value::record(Record::from_iter(entries), ctx.parser_span))
        }

        // incorrect tag
        KnownTag::Map
        | KnownTag::Str
        | KnownTag::Null
        | KnownTag::Bool
        | KnownTag::Int
        | KnownTag::Float
        | KnownTag::Binary
        | KnownTag::Set
        | KnownTag::Merge
        | KnownTag::Glob
        | KnownTag::Filesize
        | KnownTag::Duration
        | KnownTag::Range
        | KnownTag::Closure
        | KnownTag::Error
        | KnownTag::CellPath => {
            return Err(ShellError::from(ParseError::IncorrectTag {
                tag,
                at: NodeKind::Sequence,
                span: ctx.yaml_span,
            }));
        }

        // unsupported tag
        KnownTag::Timestamp | KnownTag::Value | KnownTag::Yaml => {
            return Err(ShellError::from(ParseError::UnsupportedTag {
                tag,
                at: NodeKind::Sequence,
                span: ctx.parser_span,
            }));
        }
    }
}

fn parse_mapping<'i>(
    ctx: &mut ParseCtx<'i>,
    _structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    let tag = ctx.resolve_tag(tag.as_deref())?;

    let mut values = Vec::new();
    let mut keys = HashSet::new();

    let mut merge = Record::new();
    let merge_strategy = MergeStrategy::Shallow;

    let record = 'record: loop {
        let key = 'key: loop {
            // expect a key or end
            match ctx.next_event()? {
                Event::Nothing | Event::Comment(..) => continue,
                Event::Scalar(value, scalar_style, anchor_id, tag) => {
                    let value = parse_key(ctx, value, scalar_style, tag)?;
                    if anchor_id != 0 {
                        return Err(ShellError::from(ParseError::UnexpectedKeyAnchor {
                            span: ctx.yaml_span,
                        }));
                    }
                    break 'key value;
                }
                Event::MappingEnd => {
                    let record = Record::from_iter(values);
                    break 'record merge.merge(record, merge_strategy, ctx.parser_span)?;
                }
                Event::MappingStart(..) => {
                    return Err(ShellError::from(ParseError::UnsupportedKey {
                        attempted: None,
                        ty: Type::record(),
                        span: ctx.parser_span,
                    }));
                }
                Event::SequenceStart(..) => {
                    return Err(ShellError::from(ParseError::UnsupportedKey {
                        attempted: None,
                        ty: Type::list(Type::Any),
                        span: ctx.parser_span,
                    }));
                }
                event => return Err(ctx.unexpected_event(event).into()),
            }
        };

        let value = 'value: loop {
            // expect a value
            match ctx.next_event()? {
                Event::Nothing | Event::Comment(..) => continue,
                Event::Alias(anchor_id) => break 'value ctx.get_anchor(ctx.alias(anchor_id)?)?,
                Event::Scalar(value, scalar_style, anchor_id, tag) => {
                    let value = parse_scalar(ctx, value, scalar_style, tag)?;
                    ctx.maybe_set_anchor(anchor_id, &value);
                    break 'value value;
                }
                Event::SequenceStart(structure_style, anchor_id, tag) => {
                    let value = parse_sequence(ctx, structure_style, tag)?;
                    ctx.maybe_set_anchor(anchor_id, &value);
                    break 'value value;
                }
                Event::MappingStart(structure_style, anchor_id, tag) => {
                    let value = parse_mapping(ctx, structure_style, tag)?;
                    ctx.maybe_set_anchor(anchor_id, &value);
                    break 'value value;
                }
                event => return Err(ctx.unexpected_event(event).into()),
            }
        };

        match key {
            MapKey::Merge => match value {
                Value::Record { val, .. } => {
                    merge = merge.merge(val.into_owned(), merge_strategy, ctx.parser_span)?;
                }
                Value::List { vals, .. } => {
                    for val in vals.into_iter().rev() {
                        let Value::Record { val, .. } = val else {
                            return Err(ShellError::from(ParseError::InvalidMergeList {
                                found: val.get_type(),
                                span: ctx.yaml_span,
                            }));
                        };

                        merge = merge.merge(val.into_owned(), merge_strategy, ctx.parser_span)?;
                    }
                }
                v => {
                    return Err(ShellError::from(ParseError::InvalidMergeType {
                        found: v.get_type(),
                        span: ctx.yaml_span,
                    }));
                }
            },

            MapKey::Normal(key) => {
                if !keys.insert(key.clone()) {
                    return Err(ShellError::from(ParseError::DuplicateKey {
                        duplicate: key,
                        span: ctx.yaml_span,
                    }));
                }

                values.push((key, value));
            }
        }
    };

    let tag = tag.unwrap_or(KnownTag::Map);
    match tag {
        KnownTag::Map => Ok(Value::record(record, ctx.parser_span)),
        KnownTag::Set => {
            let mut values = Vec::with_capacity(record.len());
            for (key, value) in record {
                match value {
                    // in a set every values has to be a null value
                    Value::Nothing { .. } => (),
                    v => {
                        return Err(ShellError::from(ParseError::SetFoundNotNull {
                            found: v.get_type(),
                            span: ctx.yaml_span,
                        }));
                    }
                }

                // technically in a set we could represent complexer values than strings but this is
                // too much work for that niche of an application
                values.push(Value::string(key, value.span()));
            }
            Ok(Value::list(values, ctx.parser_span))
        }

        // incorrect tag
        KnownTag::Seq
        | KnownTag::Str
        | KnownTag::Null
        | KnownTag::Bool
        | KnownTag::Int
        | KnownTag::Float
        | KnownTag::Binary
        | KnownTag::OMap
        | KnownTag::Pairs
        | KnownTag::Merge
        | KnownTag::Glob
        | KnownTag::Filesize
        | KnownTag::Duration
        | KnownTag::Range
        | KnownTag::Closure
        | KnownTag::Error
        | KnownTag::CellPath => {
            return Err(ShellError::from(ParseError::IncorrectTag {
                tag,
                at: NodeKind::Mapping,
                span: ctx.yaml_span,
            }));
        }

        // unsupported tag
        KnownTag::Timestamp | KnownTag::Value | KnownTag::Yaml => {
            return Err(ShellError::from(ParseError::UnsupportedTag {
                tag,
                at: NodeKind::Mapping,
                span: ctx.parser_span,
            }));
        }
    }
}

enum MapKey {
    Normal(String),
    Merge,
}

fn parse_key<'i>(
    ctx: &mut ParseCtx<'i>,
    value: Cow<'i, str>,
    scalar_style: ScalarStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<MapKey, ShellError> {
    let tag = ctx.resolve_tag(tag.as_deref())?;
    match tag {
        None => match (value.as_ref(), scalar_style, ctx.options.key_resolution) {
            ("<<", ScalarStyle::Plain, _) => Ok(MapKey::Merge),
            (_, ScalarStyle::Plain, KeyResolution::Strict) => {
                match parse_scalar_untagged(ctx, value.as_ref(), scalar_style)? {
                    Value::String { val, .. } => Ok(MapKey::Normal(val)),
                    v => Err(ShellError::from(ParseError::UnsupportedKey {
                        attempted: Some(value.to_string()),
                        ty: v.get_type(),
                        span: ctx.parser_span,
                    })),
                }
            }
            _ => Ok(MapKey::Normal(value.to_string())),
        },
        Some(tag) => match tag {
            KnownTag::Str => Ok(MapKey::Normal(value.to_string())),
            KnownTag::Merge => Ok(MapKey::Merge),

            // According to spec a key node may be just about everything:
            // https://yaml.org/spec/1.2.2/#mapping
            // However nushell is only able to represent mappings via `Record`,
            // therefore we enforce keys as strings.
            _ => Err(ShellError::from(ParseError::UnsupportedTag {
                tag,
                at: NodeKind::Key,
                span: ctx.parser_span,
            })),
        },
    }
}

macro_rules! parse_int {
    ($re:literal, $base:literal, $spec:literal) => {
        pastey::paste! {
            static [<INT_BASE $base>]: ::std::sync::LazyLock<::regex::Regex> =
                ::std::sync::LazyLock::new(|| ::regex::Regex::new($re)
                    .expect(concat!("valid int base ", $base, " regex for spec v", $spec)
            ));

            fn [<parse_int_base $base>]<'i>(
                ctx: &mut crate::yaml::parse::ParseCtx<'i>,
                input: &str,
                caps: ::regex::Captures<'_>
            ) -> Result<i64, crate::yaml::error::ParseError<'i>> {
                let sign = match &caps["sign"] {
                    "+" | "" => 1,
                    "-" => -1,
                    _ => unreachable!(r#"regex only matches "+", "-" or "" here"#)
                };

                let digits = &caps["digits"];
                let digits = match digits.contains("_") {
                    false => ::std::borrow::Cow::Borrowed(digits),
                    true => ::std::borrow::Cow::Owned(digits.replace("_", ""))
                };

                i64::from_str_radix(&digits, $base)
                    .map_err(|err| crate::yaml::error::ParseError::Int {
                        attempted: input.to_owned(),
                        base_and_err: Some(($base, err)),
                        span: ctx.yaml_span,
                    })
                    .map(|num| num * sign)
            }
        }
    };
}

// same for v1.1 and v1.2
mod v1_x {
    use super::*;

    pub static INFINITY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[-+]?\.(inf|Inf|INF)").expect("valid infinity regex for spec v1.x")
    });

    pub static NAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\.(nan|NaN|NAN)").expect("valid NaN regex for spec v1.x"));

    pub fn maybe_parse_null<'i>(_: &mut ParseCtx<'i>, input: &str) -> Option<()> {
        match input {
            "null" | "Null" | "NULL" | "~" | "" => Some(()),
            _ => None,
        }
    }

    pub static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
        // this regex is slightly modified from the spec to actually accept the examples
        // provided in the spec
        // https://yaml.org/type/timestamp
        Regex::new(concat!(
            "^(",                                                           // beginning
            "(?<y>[0-9][0-9][0-9][0-9])-(?<m>[0-9][0-9])-(?<d>[0-9][0-9])", // ymd
            "|(?<year>[0-9][0-9][0-9][0-9])",                               // year
            "-(?<month>[0-9][0-9]?)",                                       // month
            "-(?<day>[0-9][0-9]?)",                                         // day
            "([Tt]|[ \t]+)(?<hour>[0-9][0-9]?)",                            // hour
            ":(?<minute>[0-9][0-9])",                                       // minute
            ":(?<second>[0-9][0-9])",                                       // second
            r"(?<fraction>\.[0-9]*)?",                                      // fraction
            "([ \t]*(?:Z|(?<tz_sign>[-+])(?<tz_hour>[0-9][0-9]?)(?::(?<tz_minute>[0-9][0-9]))?))?", // time zone
            ")$", // end
        ))
        .expect("valid timestamp regex for spec v1.x")
    });
    fn parse_timestamp<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
        caps: Captures<'_>,
    ) -> Result<DateTime<FixedOffset>, ParseError<'i>> {
        let year = caps
            .name("y")
            .or(caps.name("year"))
            .expect("year exists in either capture")
            .as_str()
            .parse()
            .expect("valid year, only 4 digits");
        let month = caps
            .name("m")
            .or(caps.name("month"))
            .expect("month exists in either capture")
            .as_str()
            .parse()
            .expect("valid month, only 2 digits");
        let day = caps
            .name("d")
            .or(caps.name("day"))
            .expect("day exists in either capture")
            .as_str()
            .parse()
            .expect("valid day, only 2 digits");
        let date =
            NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| ParseError::Timestamp {
                attempted: input.to_owned(),
                issue: Some(TimestampIssue::InvalidDate),
                span: ctx.yaml_span,
            })?;

        let (Some(hour), Some(minute), Some(second)) =
            (caps.name("hour"), caps.name("minute"), caps.name("second"))
        else {
            // the TIMESTAMP regex guarantees hour, minute, and second are all present or all absent
            return Ok(NaiveDateTime::new(date, NaiveTime::MIN)
                .and_utc()
                .fixed_offset());
        };

        let hour = hour
            .as_str()
            .parse()
            .expect("valid hour, only 1 or two digits");
        let minute = minute
            .as_str()
            .parse()
            .expect("valid minute, only 1 or two digits");
        let second = second
            .as_str()
            .parse()
            .expect("valid second, only 1 or 2 digits");

        let time = match caps.name("fraction") {
            None => NaiveTime::from_hms_opt(hour, minute, second),
            Some(fraction) => {
                let fraction = f64::from_str(fraction.as_str())
                    .expect("valid fraction, only 1 dot followed by digits");
                let nano = Duration::from_secs_f64(fraction).subsec_nanos();
                NaiveTime::from_hms_nano_opt(hour, minute, second, nano)
            }
        }
        .ok_or_else(|| ParseError::Timestamp {
            attempted: input.to_owned(),
            issue: Some(TimestampIssue::InvalidTime),
            span: ctx.yaml_span,
        })?;

        let datetime = NaiveDateTime::new(date, time);
        let Some(tz_hour) = caps.name("tz_hour") else {
            return Ok(datetime.and_utc().fixed_offset());
        };

        let mut offset = Duration::from_hours(
            tz_hour
                .as_str()
                .parse()
                .expect("valid hour, only 1 or 2 digits"),
        );
        if let Some(tz_minute) = caps.name("tz_minute") {
            offset += Duration::from_mins(
                tz_minute
                    .as_str()
                    .parse()
                    .expect("valid minute, only 2 digits"),
            );
        }
        let offset = match caps.name("tz_sign").map(|m| m.as_str()) {
            None | Some("+") => FixedOffset::east_opt(offset.as_secs() as i32),
            Some("-") => FixedOffset::west_opt(offset.as_secs() as i32),
            _ => unreachable!("regex only matches '+' or '-'"),
        }
        .ok_or_else(|| ParseError::Timestamp {
            attempted: input.to_owned(),
            issue: Some(TimestampIssue::InvalidOffset),
            span: ctx.yaml_span,
        })?;

        // according to the docs, using `unwrap` is safe for `FixedOffset`
        Ok(datetime.and_local_timezone(offset).unwrap())
    }

    // this is placed in v1_x as we use it for 1.2 parsing too, but not implicitly
    pub fn maybe_parse_timestamp<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
    ) -> Result<Option<DateTime<FixedOffset>>, ParseError<'i>> {
        if let Some(caps) = TIMESTAMP.captures(input) {
            return parse_timestamp(ctx, input, caps).map(|ts| Some(ts));
        }

        Ok(None)
    }

    #[cfg(test)]
    #[rstest::rstest]
    #[case::infinity(&INFINITY)]
    #[case::nan(&NAN)]
    #[case::timestamp(&TIMESTAMP)]
    fn regex_valid(#[case] re: &LazyLock<Regex>) {
        LazyLock::force(re);
    }
}

mod v1_1 {
    use super::*;

    parse_int!("^(?<sign>[-+]?)0b(?<digits>[0-1_]+)$", 2, "1.1");
    parse_int!("^(?<sign>[-+]?)0(?<digits>[0-7_]+)$", 8, "1.1");
    parse_int!("^(?<sign>[-+]?)(?<digits>0|[1-9][0-9_]*)$", 10, "1.1");
    parse_int!("^(?<sign>[-+]?)0x(?<digits>[0-9a-fA-F_]+)$", 16, "1.1");

    static INT_BASE60: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("^(?<sign>[-+]?)(?<digits>[1-9][0-9_]*(:[0-5]?[0-9])+)$")
            .expect("valid int base 60 regex for spec v1.1")
    });
    fn parse_int_base60<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
        caps: Captures<'_>,
    ) -> Result<i64, ParseError<'i>> {
        let sign = match caps.name("sign").map(|m| m.as_str()).unwrap_or_default() {
            "+" | "" => 1,
            "-" => -1,
            _ => unreachable!(r#"regex only matches "+", "-" or "" here"#),
        };

        let mut sum = 0;
        let split: Vec<_> = caps["digits"].split(":").collect();
        for (pow, digits) in split.into_iter().rev().enumerate() {
            let digits = match digits.contains("_") {
                true => Cow::Owned(digits.replace("_", "")),
                false => Cow::Borrowed(digits),
            };

            let int = i64::from_str_radix(&digits, 10).map_err(|err| ParseError::Int {
                attempted: input.to_owned(),
                base_and_err: Some((60, err)),
                span: ctx.yaml_span,
            })?;
            sum += 60i64.pow(pow as u32) * int;
        }

        Ok(sum * sign)
    }

    pub fn maybe_parse_int<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
    ) -> Result<Option<i64>, ParseError<'i>> {
        Ok(Some(match input {
            s if let Some(caps) = INT_BASE2.captures(s) => parse_int_base2(ctx, s, caps)?,
            s if let Some(caps) = INT_BASE8.captures(s) => parse_int_base8(ctx, input, caps)?,
            s if let Some(caps) = INT_BASE10.captures(s) => parse_int_base10(ctx, input, caps)?,
            s if let Some(caps) = INT_BASE16.captures(s) => parse_int_base16(ctx, input, caps)?,
            s if let Some(caps) = INT_BASE60.captures(s) => parse_int_base60(ctx, input, caps)?,
            _ => return Ok(None),
        }))
    }

    static FLOAT_BASE10: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[-+]?([0-9][0-9_]*)?\.[0-9_]*([eE][-+][0-9]+)?$")
            .expect("valid float base 10 regex for spec v1.1")
    });
    fn parse_float_base10<'i>(ctx: &mut ParseCtx<'i>, input: &str) -> Result<f64, ParseError<'i>> {
        let no_underscore = match input.contains("_") {
            true => Cow::Owned(input.replace("_", "")),
            false => Cow::Borrowed(input),
        };

        f64::from_str(&no_underscore).map_err(|err| ParseError::Float {
            attempted: input.to_owned(),
            base_and_err: Some((10, err)),
            span: ctx.yaml_span,
        })
    }

    static FLOAT_BASE60: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?<sign>[-+]?)(?<digits>[0-9][0-9_]*(:[0-5]?[0-9])+\.[0-9_]*)$")
            .expect("valid float base 60 regex for spec v1.1")
    });
    fn parse_float_base60<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
        caps: Captures<'_>,
    ) -> Result<f64, ParseError<'i>> {
        let sign = match caps.name("sign").map(|m| m.as_str()).unwrap_or_default() {
            "+" | "" => 1.0,
            "-" => -1.0,
            _ => unreachable!(r#"regex only matches "+", "-" or "" here"#),
        };

        let mut sum = 0.0;
        let split: Vec<_> = caps["digits"].split(":").collect();
        for (pow, digits) in split.into_iter().rev().enumerate() {
            let digits = match digits.contains("_") {
                true => Cow::Owned(digits.replace("_", "")),
                false => Cow::Borrowed(digits),
            };

            let float = f64::from_str(&digits).map_err(|err| ParseError::Float {
                attempted: input.to_owned(),
                base_and_err: Some((10, err)),
                span: ctx.yaml_span,
            })?;
            sum += 60f64.powf(pow as f64) * float;
        }

        Ok(sum * sign)
    }

    pub fn maybe_parse_float<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
    ) -> Result<Option<f64>, ParseError<'i>> {
        Ok(Some(match input {
            s if FLOAT_BASE10.is_match(s) => parse_float_base10(ctx, s)?,
            s if let Some(caps) = FLOAT_BASE60.captures(s) => parse_float_base60(ctx, input, caps)?,
            s if v1_x::INFINITY.is_match(s) => match s.starts_with("-") {
                true => f64::NEG_INFINITY,
                false => f64::INFINITY,
            },
            s if v1_x::NAN.is_match(s) => f64::NAN,
            _ => return Ok(None),
        }))
    }

    pub fn maybe_parse_bool<'i>(_: &mut ParseCtx<'i>, input: &str) -> Option<bool> {
        Some(match input {
            "y" | "Y" | "yes" | "Yes" | "YES" => true,
            "n" | "N" | "no" | "No" | "NO" => false,
            "true" | "True" | "TRUE" => true,
            "false" | "False" | "FALSE" => false,
            "on" | "On" | "ON" => true,
            "off" | "Off" | "OFF" => false,
            _ => return None,
        })
    }

    #[cfg(test)]
    #[rstest::rstest]
    #[case::int_base2(&INT_BASE2)]
    #[case::int_base8(&INT_BASE8)]
    #[case::int_base10(&INT_BASE10)]
    #[case::int_base16(&INT_BASE16)]
    #[case::int_base60(&INT_BASE60)]
    #[case::float_base10(&FLOAT_BASE10)]
    #[case::float_base60(&FLOAT_BASE60)]
    fn regex_valid(#[case] re: &LazyLock<Regex>) {
        LazyLock::force(re);
    }
}

mod v1_2 {
    use super::*;

    parse_int!("^(?<sign>[-+]?)(?<digits>[0-9]+)$", 10, "1.2");
    parse_int!("^0o(?<digits>[0-7]+)$", 8, "1.2");
    parse_int!("^0x(?<digits>[0-9a-fA-F]+)$", 16, "1.2");

    pub fn maybe_parse_int<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
    ) -> Result<Option<i64>, ParseError<'i>> {
        Ok(Some(match input {
            s if let Some(caps) = INT_BASE8.captures(s) => parse_int_base8(ctx, input, caps)?,
            s if let Some(caps) = INT_BASE10.captures(s) => parse_int_base10(ctx, input, caps)?,
            s if let Some(caps) = INT_BASE16.captures(s) => parse_int_base16(ctx, input, caps)?,
            _ => return Ok(None),
        }))
    }

    static FLOAT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$")
            .expect("valid float regex for spec v1.2")
    });
    fn parse_float<'i>(ctx: &mut ParseCtx<'i>, input: &str) -> Result<f64, ParseError<'i>> {
        f64::from_str(input).map_err(|err| ParseError::Float {
            attempted: input.to_owned(),
            base_and_err: Some((10, err)),
            span: ctx.yaml_span,
        })
    }

    pub fn maybe_parse_float<'i>(
        ctx: &mut ParseCtx<'i>,
        input: &str,
    ) -> Result<Option<f64>, ParseError<'i>> {
        Ok(Some(match input {
            s if FLOAT.is_match(s) => parse_float(ctx, s)?,
            s if v1_x::INFINITY.is_match(s) => match s.starts_with("-") {
                true => f64::NEG_INFINITY,
                false => f64::INFINITY,
            },
            s if v1_x::NAN.is_match(s) => f64::NAN,
            _ => return Ok(None),
        }))
    }

    pub fn maybe_parse_bool<'i>(_: &mut ParseCtx<'i>, input: &str) -> Option<bool> {
        Some(match input {
            "true" | "True" | "TRUE" => true,
            "false" | "False" | "FALSE" => false,
            _ => return None,
        })
    }

    #[cfg(test)]
    #[rstest::rstest]
    #[case::int_base8(&INT_BASE8)]
    #[case::int_base10(&INT_BASE10)]
    #[case::int_base16(&INT_BASE16)]
    #[case::float(&FLOAT)]
    fn regex_valid(#[case] re: &LazyLock<Regex>) {
        LazyLock::force(re);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::*;
    use miette::Diagnostic;
    use nu_protocol::{IntoSpanned, test_list, test_record, test_table};
    use nu_test_support::prelude::*;
    use rstest::*;

    const FIXTURE: &str = include_str!("../../../../tests/fixtures/formats/yaml/sample.yaml");
    const SPAN: Span = Span::test_data();
    const NULL: () = ();

    #[test]
    fn parse_fixture_properly() -> Result {
        let yaml = FIXTURE.into_spanned(SPAN);
        let options = ParseOptions::default();
        parse(yaml, SPAN, options)?;
        Ok(())
    }

    #[rstest]
    #[case::double_curly_braces_with_quotes(
        r#"value: "{{ something }}""#, 
        test_record! { "value" => "{{ something }}" }
    )]
    fn parse_problematic(#[case] input: &str, #[case] expected: Value) -> Result {
        let yaml = input.into_spanned(SPAN);
        let options = ParseOptions::default();
        let parsed = parse(yaml, SPAN, options)?;
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[rstest]
    #[case::unsupported_key_mapping(
        "value: {{ something }}",
        "shell::yaml::parse::unsupported_key"
    )]
    #[case::unsupported_key_sequence(
        "value: {[ something ]}",
        "shell::yaml::parse::unsupported_key"
    )]
    fn parse_error(#[case] input: &str, #[case] expected: &str) {
        let yaml = input.into_spanned(SPAN);
        let options = ParseOptions::default();
        let err = parse(yaml, SPAN, options).unwrap_err();
        let code = err.code().unwrap().to_string();
        assert_eq!(code, expected);
    }

    #[test]
    fn test_consistent_mapping_ordering() -> Result {
        let test_yaml = Spanned {
            span: SPAN,
            item: indoc! {"
                - a: b
                  b: c
                - a: g
                  b: h
            "},
        };

        let expected = vec![
            test_record! {
                "a" => "b",
                "b" => "c",
            },
            test_record! {
                "a" => "g",
                "b" => "h"
            },
        ];

        // In a previous implementation the ordering of columns was non-deterministic.
        // It would take a few executions of the YAML conversion to see this ordering difference.
        // This loop should be fare more than enough to catch a regression.
        for i in 1..1000 {
            let parsed = parse(test_yaml, SPAN, ParseOptions::default())?;
            // Unfortunately the `eq`` function for `Value`` doesn't compare well enough to detect
            // ordering errors in List columns or values.

            let parsed = parsed.into_list()?;
            assert_eq!(expected.len(), parsed.len(), "iteration {i}");

            for (j, expected) in expected.iter().enumerate() {
                let expected = expected.as_record()?;
                let parsed = parsed[j].as_record()?;
                assert!(
                    expected.columns().eq(parsed.columns()),
                    "record {j}, iteration {i}"
                );
                assert!(
                    expected.values().eq(parsed.values()),
                    "record {j}, iteration {i}"
                );
            }
        }

        Ok(())
    }

    #[rstest]
    #[case("Key: !Value ${TEST}-Test-role")]
    #[case("Key: !Value test-${TEST}")]
    #[case("Key: !Value")]
    #[case("Key: !True")]
    #[case("Key: !123")]
    fn ignore_unknown_tags(#[case] input: &str) {
        let yaml = input.into_spanned(SPAN);
        let options = ParseOptions::default().with_ignore_tags(true);
        assert!(parse(yaml, SPAN, options).is_ok());
    }

    fn parse_yaml_v1_1<T: FromValue>(input: &str) -> Result<T> {
        let yaml = input.into_spanned(SPAN);
        let options = ParseOptions::default()
            .with_spec(Spec::V1_1)
            .with_key_resolution(KeyResolution::Verbatim);
        let parsed = parse(yaml, SPAN, options)?;
        Ok(T::from_value(parsed)?)
    }

    #[test]
    fn spec_type_binary() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/binary.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        let expected = include_bytes!("../../../../tests/fixtures/formats/yaml/binary.gif");
        assert_eq!(record["canonical"].as_binary()?, expected);
        assert_eq!(record["generic"].as_binary()?, expected);
        assert_eq!(
            record["description"].as_str()?,
            "The binary value above is a tiny arrow encoded as a gif image."
        );

        Ok(())
    }

    #[test]
    fn spec_type_bool() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/bool.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        assert_eq!(record["canonical"].as_bool()?, true);
        assert_eq!(record["answer"].as_bool()?, false);
        assert_eq!(record["logical"].as_bool()?, true);
        assert_eq!(record["option"].as_bool()?, true);
        Ok(())
    }

    #[test]
    fn spec_type_float() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/float.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        assert_eq!(record["canonical"].as_float()?, 6.8523015e+5);
        assert_eq!(record["exponential"].as_float()?, 685.230_15e+03);
        assert_eq!(record["fixed"].as_float()?, 685_230.15);
        assert_eq!(
            record["sexagesimal"].as_float()?,
            190. * 60. * 60. + 20. * 60. + 30.15
        );
        assert_eq!(record["negative infinity"].as_float()?, f64::NEG_INFINITY);
        assert!(record["not a number"].as_float()?.is_nan());
        Ok(())
    }

    #[test]
    fn spec_type_int() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/int.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        assert_eq!(record["canonical"].as_int()?, 685230);
        assert_eq!(record["decimal"].as_int()?, 685_230);
        assert_eq!(record["octal"].as_int()?, 0o2472256);
        assert_eq!(record["hexadecimal"].as_int()?, 0x_0A_74_AE);
        assert_eq!(record["binary"].as_int()?, 0b1010_0111_0100_1010_1110);
        assert_eq!(
            record["sexagesimal"].as_int()?,
            190 * 60 * 60 + 20 * 60 + 30
        );
        Ok(())
    }

    #[test]
    fn spec_type_map() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/map.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        let block_style = record["Block style"].as_record()?;
        assert_eq!(block_style["Clark"].as_str()?, "Evans");
        assert_eq!(block_style["Brian"].as_str()?, "Ingerson");
        assert_eq!(block_style["Oren"].as_str()?, "Ben-Kiki");
        let flow_style = record["Flow style"].as_record()?;
        assert_eq!(flow_style["Clark"].as_str()?, "Evans");
        assert_eq!(flow_style["Brian"].as_str()?, "Ingerson");
        assert_eq!(flow_style["Oren"].as_str()?, "Ben-Kiki");
        Ok(())
    }

    #[test]
    fn spec_type_merge() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/merge.yaml");
        let list: Vec<Value> = dbg!(parse_yaml_v1_1(yaml)?);
        let [
            center,
            left,
            big,
            small,
            explicit_keys,
            merge_one_map,
            merge_multiple_maps,
            override_,
        ] = list.try_into().unwrap();

        assert_eq!(center, test_record! { "x" => 1, "y" => 2 }, "CENTER");
        assert_eq!(left, test_record! { "x" => 0, "y" => 2 }, "LEFT");
        assert_eq!(big, test_record! { "r" => 10 }, "BIG");
        assert_eq!(small, test_record! { "r" => 1 }, "SMALL");

        assert_eq!(
            explicit_keys,
            test_record! {
                "x" => 1,
                "y" => 2,
                "r" => 10,
                "label" => "center/big",
            },
            "Explicit keys"
        );

        assert_eq!(
            merge_one_map,
            test_record! {
                "x" => 1,
                "y" => 2,
                "r" => 10,
                "label" => "center/big",
            },
            "Merge one map"
        );

        assert_eq!(
            merge_multiple_maps,
            test_record! {
                "x" => 1,
                "y" => 2,
                "r" => 10,
                "label" => "center/big",
            },
            "Merge multiple maps"
        );

        assert_eq!(
            override_,
            test_record! {
                "r" => 10,
                "x" => 1,
                "y" => 2,
                "label" => "center/big",
            },
            "Override"
        );

        Ok(())
    }

    #[test]
    fn spec_type_null() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/null.yaml");
        let documents: Vec<Value> = parse_yaml_v1_1(yaml)?;
        let [null, mapping, sparse] = documents.try_into().unwrap();

        assert!(null.is_nothing());

        assert_eq!(
            mapping,
            test_record! {
                "empty" => NULL,
                "canonical" => NULL,
                "english" => NULL,
                "~" => "null key"
            }
        );

        assert_eq!(
            sparse.as_record()?["sparse"],
            test_list![NULL, "2nd entry", NULL, "4th entry", NULL]
        );

        Ok(())
    }

    #[test]
    fn spec_type_omap() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/omap.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;

        assert_eq!(
            record["Bestiary"],
            test_record! {
                "aardvark" => "African pig-like ant eater. Ugly.",
                "anteater" => "South-American ant eater. Two species.",
                "anaconda" => "South-American constrictor snake. Scaly.",
            }
        );

        assert_eq!(
            record["Numbers"],
            test_record! {
                "one" => 1,
                "two" => 2,
                "three" => 3,
            }
        );

        Ok(())
    }

    #[test]
    fn spec_type_pairs() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/pairs.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;

        assert_eq!(
            record["Block tasks"],
            test_table![
                ["key", "value"];
                ["meeting", "with team."],
                ["meeting", "with boss."],
                ["break", "lunch."],
                ["meeting", "with client."]
            ]
        );

        assert_eq!(
            record["Flow tasks"],
            test_table![
                ["key", "value"];
                ["meeting", "with team"],
                ["meeting", "with boss"]
            ]
        );

        Ok(())
    }

    #[test]
    fn spec_type_seq() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/seq.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;

        assert_eq!(
            record["Block style"],
            test_list![
                "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
                "Pluto",
            ]
        );

        assert_eq!(
            record["Flow style"],
            test_list![
                "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
                "Pluto",
            ]
        );

        Ok(())
    }

    #[test]
    fn spec_type_set() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/set.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;

        assert_eq!(
            record["baseball players"],
            test_list!["Mark McGwire", "Sammy Sosa", "Ken Griffey",]
        );

        assert_eq!(
            record["baseball teams"],
            test_list!["Boston Red Sox", "Detroit Tigers", "New York Yankees"]
        );

        Ok(())
    }

    #[test]
    fn spec_type_str() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/str.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;
        assert_eq!(record["string"].as_str()?, "abcd");
        Ok(())
    }

    #[test]
    fn spec_type_timestamp() -> Result {
        let yaml = include_str!("../../../../tests/fixtures/formats/yaml/timestamp.yaml");
        let record: Record = parse_yaml_v1_1(yaml)?;

        assert_eq!(
            record["canonical"].as_date()?,
            "2001-12-15T02:59:43.1Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap()
        );
        assert_eq!(
            record["valid iso8601"].as_date()?,
            "2001-12-14T21:59:43.10-05:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap()
        );
        assert_eq!(
            record["space separated"].as_date()?,
            "2001-12-14T21:59:43.10-05:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap()
        );
        assert_eq!(
            record["no time zone (Z)"].as_date()?,
            "2001-12-15T02:59:43.10Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap()
        );
        assert_eq!(
            record["date (00:00:00Z)"].as_date()?,
            "2002-12-14T00:00:00Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap()
        );

        Ok(())
    }
}
