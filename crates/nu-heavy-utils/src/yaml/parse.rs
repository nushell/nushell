// Throughout these functions, there are two spans, the yaml_span which is the input value and the
// parser_span which is the command that does the parsing.
// All errors that occur through bad parsing or of lack of implementing a yaml feature should refer
// to the parser_span, all errors that are caused by the value as it is an incorrect yaml, should
// use the yaml_span.

use crate::yaml::Spec;
use derive_setters::Setters;
use serde_saphyr::granit_parser::{Event, Parser, ScalarStyle, StrInput, StructureStyle, Tag};
use nu_protocol::{Record, ShellError, Span, Spanned, Value, shell_error::generic::GenericError};
use regex::Regex;
use std::{borrow::Cow, collections::{HashMap, HashSet}, num::NonZeroUsize, str::FromStr, sync::LazyLock};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct ParseOptions {
    keep_styles: bool,
    multiple: ParseMultiple,
    spec: Spec,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ParseMultiple {
    #[default]
    Auto,
    ForceList,
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
        event => return Err(ctx.unexpected_event(event)),
    }

    let mut documents = Vec::new();
    loop {
        match ctx.next_event()? {
            Event::DocumentStart(_) => documents.push(parse_document(ctx)?),
            Event::StreamEnd => break,
            Event::Nothing | Event::Comment(..) => continue,
            event => return Err(ctx.unexpected_event(event)),
        }
    }

    use ParseMultiple as PM;
    let value = match (ctx.options.multiple, documents.len()) {
        (PM::Auto | PM::ForceSingle, 0) => todo!("handle no document"),
        (PM::Auto | PM::ForceSingle, 1) => documents.into_iter().next().expect("non-empty"),
        (PM::Auto | PM::ForceList, _) => Value::list(documents, ctx.parser_span),
        (PM::ForceSingle, n) => todo!("handle force single when more than 1 document"),
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
    fn next_event(&mut self) -> Result<Event<'i>, ShellError> {
        match self.parser.next_event() {
            None => Err(ShellError::Generic(
                GenericError::new(
                    "Unexpected end of YAML events",
                    "Unexpectedly the event stream of the YAML parser ended",
                    self.parser_span,
                )
                .with_code("shell::yaml::parser::end_of_events")
                .with_help("This is most likely a bug. Please report it."),
            )),
            Some(Err(err)) => Err(ShellError::Generic(
                GenericError::new(
                    "Scanning YAML failed",
                    "Scanning the YAML input failed",
                    self.yaml_span,
                )
                .with_code("shell::yaml::parser::scan_error")
                .with_source(err),
            )),
            Some(Ok((event, _))) => Ok(event),
        }
    }

    #[track_caller]
    fn unexpected_event(&self, event: Event<'i>) -> ShellError {
        ShellError::Generic(
            GenericError::new(
                "Internal YAML Parser Error",
                "The YAML parser got into an unexpected state",
                self.parser_span,
            )
            .with_code("shell::yaml::parser::internal")
            .with_help("This is most likely a bug. Please report it.")
            .with_inner([ShellError::Generic(
                GenericError::new_internal(
                    "Unexpected YAML event",
                    format!("Unexpected YAML event during parsing: {event:?}"),
                )
                .with_code("shell::yaml::parser::unexpected_event"),
            )]),
        )
    }

    fn unexpected_key_anchor(&self) -> ShellError {
        todo!()
    }

    fn unhandled_tags(&self, tag: Option<Cow<'_, Tag>>) -> Result<(), ShellError> {
        match tag {
            None => Ok(()),
            Some(tag) => Err(ShellError::Generic(
                GenericError::new(
                    "Tags not supported",
                    "The current implementation does not support tags yet",
                    self.parser_span,
                )
                .with_code("shell::yaml::parser::unsupported_tags")
                .with_inner([GenericError::new(
                    "Unsupported tag",
                    format!("The tag {tag:?} is not supported"),
                    self.yaml_span,
                )
                .into()]),
            )),
        }
    }

    fn alias(&self, id: usize) -> Result<NonZeroUsize, ShellError> {
        NonZeroUsize::new(id).ok_or(ShellError::Generic(
            GenericError::new(
                "Invalid Alias ID",
                "YAML parser generated 0 as an Alias ID",
                self.parser_span,
            )
            .with_code("shell::yaml::parser::zero_alias")
            .with_help("This error should not occur and is likely a bug. Please report it."),
        ))
    }

    fn set_anchor(&mut self, anchor_id: NonZeroUsize, value: Value) {
        self.anchors.insert(anchor_id, value);
    }

    fn maybe_set_anchor(&mut self, anchor_id: usize, value: &Value) {
        NonZeroUsize::new(anchor_id).map(|anchor_id| self.set_anchor(anchor_id, value.clone()));
    }

    fn get_anchor(&self, anchor_id: NonZeroUsize) -> Result<Value, ShellError> {
        match self.anchors.get(&anchor_id) {
            Some(value) => Ok(value.clone()),
            None => todo!(),
        }
    }
}

fn parse_document<'i>(ctx: &mut ParseCtx<'i>) -> Result<Value, ShellError> {
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
            event => return Err(ctx.unexpected_event(event)),
        }
    };

    loop {
        match ctx.next_event()? {
            Event::Nothing | Event::Comment(..) => continue,
            Event::DocumentEnd => return Ok(value),
            event => return Err(ctx.unexpected_event(event)),
        }
    }
}

static BASE10: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[-+]?[0-9]+$").expect("valid base 10 regex"));
fn parse_base10(ctx: &mut ParseCtx<'_>, s: &str) -> Result<i64, ShellError> {
    i64::from_str_radix(s, 10).map_err(|err| {
        ShellError::Generic(
            GenericError::new(
                "Parsing Base 10 failed",
                format!("Parsing {s:?} failed, {err}"),
                ctx.yaml_span,
            )
            .with_code("shell::yaml::parser::num::base10"),
        )
    })
}

static BASE8: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^0o[0-7]+$").expect("valid base 8 regex"));
fn parse_base8(ctx: &mut ParseCtx<'_>, s: &str) -> Result<i64, ShellError> {
    let (_, digits) = s.split_at(b"0o".len());
    i64::from_str_radix(digits, 8).map_err(|err| {
        ShellError::Generic(
            GenericError::new(
                "Parsing Base 8 failed",
                format!("Parsing {s:?} failed, {err}"),
                ctx.yaml_span,
            )
            .with_code("shell::yaml::parser::num::base8"),
        )
    })
}

static BASE16: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^0x[0-9a-fA-F]+$").expect("valid base 16 regex"));
fn parse_base16(ctx: &mut ParseCtx<'_>, s: &str) -> Result<i64, ShellError> {
    let (_, digits) = s.split_at(b"0x".len());
    i64::from_str_radix(digits, 16).map_err(|err| {
        ShellError::Generic(
            GenericError::new(
                "Parsing Base 16 failed",
                format!("Parsing {s:?} failed, {err}"),
                ctx.yaml_span,
            )
            .with_code("shell::yaml::parser::num::base16"),
        )
    })
}

static FLOAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$").expect("valid float regex")
});
fn parse_float(ctx: &mut ParseCtx<'_>, s: &str) -> Result<f64, ShellError> {
    f64::from_str(s).map_err(|err| {
        ShellError::Generic(
            GenericError::new(
                "Parsing Float failed",
                format!("Parsing {s:?} failed, {err}"),
                ctx.yaml_span,
            )
            .with_code("shell::yaml::parser::num::float"),
        )
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
    ctx.unhandled_tags(tag)?;

    let span = ctx.parser_span;

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
    Ok(match value.as_ref() {
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

// gets called on Event::SequenceStart, returns on Event::SequenceEnd
// returns Value::List
fn parse_sequence<'i>(
    ctx: &mut ParseCtx<'i>,
    _structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    ctx.unhandled_tags(tag)?;

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
            Event::SequenceEnd => return Ok(Value::list(values, ctx.parser_span)),
            event => return Err(ctx.unexpected_event(event)),
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
    ctx.unhandled_tags(tag)?;

    let mut values = Vec::new();
    let mut keys = HashSet::new();

    loop {
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
                    return Ok(Value::record(Record::from_iter(values), ctx.parser_span));
                }
                event => return Err(ctx.unexpected_event(event)),
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
                event => return Err(ctx.unexpected_event(event)),
            }
        };

        if !keys.insert(key.clone()) {
            return Err(ShellError::Generic(
                GenericError::new(
                    "Duplicate YAML Key",
                    format!("The key {key:?} already appeared in the mapping"),
                    ctx.yaml_span,
                )
                .with_code("shell::yaml::parse::duplicate_key"),
            ));
        }

        values.push((key, value))
    }
}

fn parse_key<'i>(
    ctx: &mut ParseCtx<'i>,
    value: Cow<'i, str>,
    _: ScalarStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<String, ShellError> {
    ctx.unhandled_tags(tag)?;

    // According to spec a key node may be just about everything:
    // https://yaml.org/spec/1.2.2/#mapping
    // However nushell is only able to represent mappings via `Record`,
    // therefore we enforce keys as strings.

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::IntoSpanned;
    use nu_test_support::prelude::*;

    const FIXTURE: &str = include_str!("../../../../tests/fixtures/formats/sample.yaml");
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
