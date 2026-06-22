// Throughout these functions, there are two spans, the yaml_span which is the input value and the
// parser_span which is the command that does the parsing.
// All errors that occur through bad parsing or of lack of implementing a yaml feature should refer
// to the parser_span, all errors that are caused by the value as it is an incorrect yaml, should
// use the yaml_span.

use crate::{
    merge::{Merge, MergeStrategy},
    yaml::{
        KnownTag, Spec, UnknownTagError,
        error::{InternalParserError, ParseError},
    },
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::DateTime;
use derive_setters::Setters;
use nu_protocol::{
    FromValue, Range, Record, ShellError, Span, Spanned, Value, ast::CellPath, record,
    shell_error::generic::GenericError,
};
use nu_utils::location::Location;
use regex::Regex;
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

    fn unexpected_key_anchor(&self) -> ShellError {
        todo!()
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

static BASE10: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[-+]?[0-9]+$").expect("valid base 10 regex"));
fn parse_base10<'i>(ctx: &mut ParseCtx<'i>, s: &str) -> Result<i64, ParseError<'i>> {
    i64::from_str_radix(s, 10).map_err(|err| ParseError::NumInt {
        base: 10,
        attempted: s.to_owned(),
        err,
        span: ctx.yaml_span,
    })
}

static BASE8: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^0o[0-7]+$").expect("valid base 8 regex"));
fn parse_base8<'i>(ctx: &mut ParseCtx<'i>, s: &str) -> Result<i64, ParseError<'i>> {
    let (_, digits) = s.split_at(b"0o".len());
    i64::from_str_radix(digits, 8).map_err(|err| ParseError::NumInt {
        base: 8,
        attempted: s.to_owned(),
        err,
        span: ctx.yaml_span,
    })
}

static BASE16: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^0x[0-9a-fA-F]+$").expect("valid base 16 regex"));
fn parse_base16<'i>(ctx: &mut ParseCtx<'i>, s: &str) -> Result<i64, ParseError<'i>> {
    let (_, digits) = s.split_at(b"0x".len());
    i64::from_str_radix(digits, 16).map_err(|err| ParseError::NumInt {
        base: 16,
        attempted: s.to_owned(),
        err,
        span: ctx.yaml_span,
    })
}

static FLOAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$").expect("valid float regex")
});
fn parse_float<'i>(ctx: &mut ParseCtx<'i>, s: &str) -> Result<f64, ParseError<'i>> {
    f64::from_str(s).map_err(|err| ParseError::NumFloat {
        attempted: s.to_owned(),
        err,
        span: ctx.yaml_span,
    })
}

static INFINITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[-+]?(\.inf|\.Inf|\.INF)$").expect("valid infinity regex"));

static NAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\.nan|\.NaN|\.NAN)$").expect("valid NaN regex"));

// parse the scalar, this one has to figure out how what type the value might be
fn parse_scalar<'i>(
    ctx: &mut ParseCtx<'i>,
    value: Cow<'i, str>,
    scalar_style: ScalarStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    // TODO: make a single function for this
    let tag = match ctx.options.ignore_tags {
        true => None,
        false => tag
            .map(|tag| KnownTag::from_str(tag.to_string().as_str()))
            .transpose()
            .map_err(|err| ctx.unknown_tag_err(err))?,
    };

    // TODO: handle spec 1.1 and 1.2

    let span = ctx.parser_span;
    let value = value.as_ref();

    match tag {
        None => {
            match scalar_style {
                ScalarStyle::Plain => (),

                // Without tags, these can only be strings.
                ScalarStyle::SingleQuoted
                | ScalarStyle::DoubleQuoted
                | ScalarStyle::Literal
                | ScalarStyle::Folded => return Ok(Value::string(value, span)),
            }

            // We resolve values according to the core schema
            // https://yaml.org/spec/1.2.2/#1032-tag-resolution
            Ok(match value {
                "null" | "Null" | "NULL" | "~" | "" => Value::nothing(span),
                "true" | "True" | "TRUE" => Value::bool(true, span),
                "false" | "False" | "FALSE" => Value::bool(false, span),
                s if BASE10.is_match(s) => Value::int(parse_base10(ctx, s)?, span),
                s if BASE8.is_match(s) => Value::int(parse_base8(ctx, s)?, span),
                s if BASE16.is_match(s) => Value::int(parse_base16(ctx, s)?, span),
                s if FLOAT.is_match(s) => Value::float(parse_float(ctx, s)?, span),
                s if INFINITY.is_match(s) => Value::float(f64::INFINITY, span),
                s if NAN.is_match(s) => Value::float(f64::NAN, span),
                s => Value::string(s, span),
            })
        }

        Some(tag) => Ok(match tag {
            KnownTag::Str => Value::string(value, span),
            KnownTag::Null => Value::nothing(span),
            KnownTag::Bool => match value.to_lowercase().as_ref() {
                "true" => Value::bool(true, span),
                "false" => Value::bool(false, span),
                _ => {
                    return Err(ShellError::from(ParseError::Bool {
                        attempted: value.to_owned(),
                        span: ctx.yaml_span,
                    }));
                }
            },
            KnownTag::Int if BASE8.is_match(value) => Value::int(parse_base8(ctx, value)?, span),
            KnownTag::Int if BASE16.is_match(value) => Value::int(parse_base16(ctx, value)?, span),
            KnownTag::Int => Value::int(parse_base10(ctx, value)?, span),
            KnownTag::Float if INFINITY.is_match(value) => Value::float(f64::INFINITY, span),
            KnownTag::Float if NAN.is_match(value) => Value::float(f64::NAN, span),
            KnownTag::Float => Value::float(parse_float(ctx, value)?, span),
            KnownTag::Binary => Value::binary(
                BASE64_STANDARD
                    .decode(&value)
                    .map_err(|err| ParseError::Binary {
                        attempted: value.to_owned(),
                        err,
                        span: ctx.yaml_span,
                    })?,
                span,
            ),
            KnownTag::Glob => Value::glob(value, false, span),
            KnownTag::Filesize => Value::filesize(parse_base10(ctx, value)?, span),
            KnownTag::Duration => Value::duration(parse_base10(ctx, value)?, span),
            KnownTag::Date => Value::date(
                DateTime::from_str(value).map_err(|err| ParseError::Date {
                    attempted: value.to_owned(),
                    err,
                    span: ctx.yaml_span,
                })?,
                span,
            ),
            KnownTag::Range => Value::range(
                Range::from_str(value).map_err(|err| ParseError::Range {
                    attempted: value.to_owned(),
                    err,
                    span: ctx.yaml_span,
                })?,
                span,
            ),
            KnownTag::CellPath => Value::cell_path(
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
            KnownTag::Closure => {
                return Err(ShellError::from(ParseError::UnimplementedTag { tag, span }));
            }

            // incorrect tag
            KnownTag::Map => todo!(),
            KnownTag::Seq => todo!(),
            KnownTag::OMap => todo!(),
            KnownTag::Pairs => todo!(),
            KnownTag::Set => todo!(),
            KnownTag::Merge => todo!(),
            KnownTag::Error => todo!(),

            // unsupported tag
            KnownTag::Timestamp => todo!(),
            KnownTag::Value => todo!(),
            KnownTag::Yaml => todo!(),
        }),
    }
}

// gets called on Event::SequenceStart, returns on Event::SequenceEnd
// returns Value::List
fn parse_sequence<'i>(
    ctx: &mut ParseCtx<'i>,
    _structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    let tag = match ctx.options.ignore_tags {
        true => None,
        false => tag
            .map(|tag| KnownTag::from_str(tag.to_string().as_str()))
            .transpose()
            .map_err(|err| ctx.unknown_tag_err(err))?,
    };

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

    match tag.unwrap_or(KnownTag::Seq) {
        KnownTag::Seq => Ok(Value::list(values, ctx.parser_span)),
        KnownTag::Pairs => {
            let mut pairs = Vec::with_capacity(values.len());
            for value in values.into_iter() {
                let span = value.span();
                let record = value
                    .into_record()
                    .map_err(|_| ShellError::Generic(todo!()))?;
                let mut iter = record.into_iter();
                match (iter.next(), iter.next()) {
                    (None, None) => todo!("throw error for empty record"),
                    (Some((key, value)), None) => pairs.push(Value::record(
                        record!(
                            "key" => Value::string(key, span),
                            "value" => value
                        ),
                        span,
                    )),
                    (_, Some(_)) => todo!("too many entries"),
                }
            }
            Ok(Value::list(pairs, ctx.parser_span))
        }
        KnownTag::OMap => {
            let mut entries = Vec::with_capacity(values.len());
            let mut keys = HashSet::with_capacity(values.len());
            for value in values.into_iter() {
                let record = value
                    .into_record()
                    .map_err(|_| ShellError::Generic(todo!()))?;
                let mut iter = record.into_iter();
                match (iter.next(), iter.next()) {
                    (None, None) => todo!("too few entries"),
                    (Some((key, value)), None) => {
                        if !keys.insert(key.clone()) {
                            todo!("throw error for duplicate key");
                        }

                        entries.push((key, value))
                    }
                    (_, Some(_)) => todo!("too many entries"),
                }
            }
            Ok(Value::record(Record::from_iter(entries), ctx.parser_span))
        }

        KnownTag::Map => todo!(),
        KnownTag::Str => todo!(),
        KnownTag::Null => todo!(),
        KnownTag::Bool => todo!(),
        KnownTag::Int => todo!(),
        KnownTag::Float => todo!(),
        KnownTag::Binary => todo!(),
        KnownTag::Set => todo!(),
        KnownTag::Merge => todo!(),
        KnownTag::Timestamp => todo!(),
        KnownTag::Value => todo!(),
        KnownTag::Yaml => todo!(),
        KnownTag::Glob => todo!(),
        KnownTag::Filesize => todo!(),
        KnownTag::Duration => todo!(),
        KnownTag::Date => todo!(),
        KnownTag::Range => todo!(),
        KnownTag::Closure => todo!(),
        KnownTag::Error => todo!(),
        KnownTag::CellPath => todo!(),
    }
}

// gets called on Event::MappingStart, returns on Event::MappingEnd
// returns Value::Record
fn parse_mapping<'i>(
    ctx: &mut ParseCtx<'i>,
    _structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    let tag = match ctx.options.ignore_tags {
        true => None,
        false => tag
            .map(|tag| KnownTag::from_str(tag.to_string().as_str()))
            .transpose()
            .map_err(|err| ctx.unknown_tag_err(err))?,
    };

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
                        return Err(ctx.unexpected_key_anchor());
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
                            todo!("throw error explaining merge is not a record")
                        };

                        merge = merge.merge(val.into_owned(), merge_strategy, ctx.parser_span)?;
                    }
                }
                _ => todo!("throw invalid merge objects"),
            },

            MapKey::Normal(key) => {
                if !keys.insert(key.clone()) {
                    return Err(ShellError::Generic(
                        GenericError::new(
                            "Duplicate YAML Key",
                            format!("The key {key:?} already appeared in the mapping"),
                            ctx.yaml_span,
                        )
                        .with_code("shell::yaml::parser::duplicate_key"),
                    ));
                }

                values.push((key, value));
            }
        }
    };

    match tag.unwrap_or(KnownTag::Map) {
        KnownTag::Map => Ok(Value::record(record, ctx.parser_span)),
        KnownTag::Set => {
            let mut values = Vec::with_capacity(record.len());
            for (key, value) in record {
                match value {
                    // in a set every values has to be a null value
                    Value::Nothing { .. } => (),
                    _ => todo!("throw error for not null value"),
                }

                // technically in a set we could represent complexer values than strings but this is
                // too much work for that niche of an application
                values.push(Value::string(key, value.span()));
            }
            Ok(Value::list(values, ctx.parser_span))
        }

        KnownTag::Seq => todo!(),
        KnownTag::Str => todo!(),
        KnownTag::Null => todo!(),
        KnownTag::Bool => todo!(),
        KnownTag::Int => todo!(),
        KnownTag::Float => todo!(),
        KnownTag::Binary => todo!(),
        KnownTag::OMap => todo!(),
        KnownTag::Pairs => todo!(),
        KnownTag::Merge => todo!(),
        KnownTag::Timestamp => todo!(),
        KnownTag::Value => todo!(),
        KnownTag::Yaml => todo!(),
        KnownTag::Glob => todo!(),
        KnownTag::Filesize => todo!(),
        KnownTag::Duration => todo!(),
        KnownTag::Date => todo!(),
        KnownTag::Range => todo!(),
        KnownTag::Closure => todo!(),
        KnownTag::Error => todo!(),
        KnownTag::CellPath => todo!(),
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
    // ctx.unhandled_tags(tag)?;

    // According to spec a key node may be just about everything:
    // https://yaml.org/spec/1.2.2/#mapping
    // However nushell is only able to represent mappings via `Record`,
    // therefore we enforce keys as strings.

    Ok(match (value.as_ref(), scalar_style) {
        ("<<", ScalarStyle::Plain) => MapKey::Merge,
        _ => MapKey::Normal(value.to_string()),
    })
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
