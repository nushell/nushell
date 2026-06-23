// Throughout these functions, there are two spans, the yaml_span which is the input value and the
// parser_span which is the command that does the parsing.
// All errors that occur through bad parsing or of lack of implementing a yaml feature should refer
// to the parser_span, all errors that are caused by the value as it is an incorrect yaml, should
// use the yaml_span.

use crate::{
    merge::{Merge, MergeStrategy},
    yaml::{
        KnownTag, Spec, UnknownTagError,
        error::{InternalParserError, NodeKind, ParseError},
    },
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::DateTime;
use derive_setters::Setters;
use nu_protocol::{
    FromValue, Range, Record, ShellError, Span, Spanned, Value, ast::CellPath, record,
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
};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct ParseOptions {
    keep_styles: bool,
    multiple: ParseMultiple,
    spec: Spec,
    ignore_tags: bool,
}

#[derive(Debug, Clone, Copy, Default, FromValue)]
pub enum ParseMultiple {
    #[default]
    #[nu_value(rename = "auto")]
    Auto,

    #[nu_value(rename = "list")]
    ForceList,

    #[nu_value(rename = "single")]
    ForceSingle,
}

pub fn parse(yaml: Spanned<&str>, span: Span, options: &ParseOptions) -> Result<Value, ShellError> {
    let parser = Parser::new_from_str(yaml.item);
    let ctx = &mut ParseCtx {
        parser,
        parser_span: span,
        yaml_span: yaml.span,
        anchors: HashMap::new(),
        options,
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

// parse the scalar, this one has to figure out how what type the value might be
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
                .decode(&value)
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
        (KnownTag::Date, _) => Value::date(
            DateTime::from_str(value).map_err(|err| ParseError::Date {
                attempted: value.to_owned(),
                err,
                span: ctx.yaml_span,
            })?,
            span,
        ),
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
        (KnownTag::Closure, _) => {
            return Err(ShellError::from(ParseError::UnimplementedTag { tag, span }));
        }

        // incorrect tag
        (
            KnownTag::Map
            | KnownTag::Seq
            | KnownTag::OMap
            | KnownTag::Pairs
            | KnownTag::Set
            | KnownTag::Merge
            | KnownTag::Error,
            _,
        ) => {
            return Err(ShellError::from(ParseError::IncorrectTag {
                tag,
                at: NodeKind::Scalar,
                span: ctx.yaml_span,
            }));
        }

        // unsupported tag
        (KnownTag::Timestamp | KnownTag::Value | KnownTag::Yaml, _) => {
            return Err(ShellError::from(ParseError::UnsupportedTag {
                tag,
                at: NodeKind::Scalar,
                span,
            }));
        }
    })
}

// gets called on Event::SequenceStart, returns on Event::SequenceEnd
// returns Value::List
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
        | KnownTag::Date
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

// gets called on Event::MappingStart, returns on Event::MappingEnd
// returns Value::Record
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
        | KnownTag::Date
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
        None => Ok(match (value.as_ref(), scalar_style) {
            ("<<", ScalarStyle::Plain) => MapKey::Merge,
            _ => MapKey::Normal(value.to_string()),
        }),
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

    #[cfg(test)]
    #[rstest::rstest]
    #[case::infinity(&INFINITY)]
    #[case::nan(&NAN)]
    fn regex_valid(#[case] re: &LazyLock<Regex>) {
        LazyLock::force(re);
    }
}

mod v1_1 {
    use super::*;

    // TODO: add reference to this once spec is available again

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
        Regex::new(r"^[-+]?([0-9][0-9_]*)?\.[0-9.]*([eE][-+][0-9]+)?$")
            .expect("valid float base 10 regex for spec v1.1")
    });
    fn parse_float_base10<'i>(ctx: &mut ParseCtx<'i>, input: &str) -> Result<f64, ParseError<'i>> {
        let no_underscore = match input.contains("_") {
            true => Cow::Owned(input.replace("_", "to")),
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

    // We resolve values according to the core schema.
    // https://yaml.org/spec/1.2.2/#1032-tag-resolution

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
    use nu_protocol::IntoSpanned;
    use nu_test_support::prelude::*;

    const FIXTURE: &str = include_str!("../../../../tests/fixtures/formats/yaml/sample.yaml");
    const SPAN: Span = Span::test_data();

    #[test]
    fn parse_fixture_properly() -> Result {
        let yaml = FIXTURE.into_spanned(SPAN);
        let options = ParseOptions::default();
        parse(yaml, SPAN, &options)?;
        Ok(())
    }

    #[test]
    fn parse_string() -> Result {
        let yaml = "🐘".into_spanned(SPAN);
        let options = ParseOptions::default();
        parse(yaml, SPAN, &options)?;
        Ok(())
    }
}
