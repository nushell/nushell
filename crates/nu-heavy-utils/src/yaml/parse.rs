// Throughout these functions, there are two spans, the yaml_span which is the input value and the
// parser_span which is the command that does the parsing.
// All errors that occur through bad parsing or of lack of implementing a yaml feature should refer
// to the parser_span, all errors that are caused by the value as it is an incorrect yaml, should
// use the yaml_span.

use std::{borrow::Cow, collections::HashMap, num::NonZeroUsize};

use crate::yaml::Spec;
use derive_setters::Setters;
use granit_parser::{Event, Parser, ScalarStyle, ScanError, StrInput, StructureStyle, Tag};
use nu_protocol::{Record, ShellError, Span, Spanned, Value, shell_error::generic::GenericError};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct ParseOptions {
    keep_styles: bool,
    multiple: ParseMultiple,
    spec: Spec,
}

#[derive(Debug, Clone, Default)]
pub enum ParseMultiple {
    #[default]
    Auto,
    ForceList,
    ForceSingle,
}

pub fn parse(yaml: Spanned<&str>, span: Span, options: &ParseOptions) -> Result<Value, ShellError> {
    let yaml_str = yaml.item;
    let yaml_span = yaml.span;
    let mut parser = Parser::new_from_str(yaml_str);
    todo!()
}

struct ParseCtx<'i> {
    parser: Parser<'i, StrInput<'i>>,
    parser_span: Span,
    yaml_span: Span,
    anchors: HashMap<NonZeroUsize, Value>,
    options: ParseOptions,
}

impl<'i> ParseCtx<'i> {
    fn next_event(&mut self) -> Result<Event<'i>, ShellError> {
        todo!()
    }

    fn unexpected_event(&self, event: Event<'i>) -> ShellError {
        todo!()
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

    fn get_anchor(&self, anchor_id: NonZeroUsize) -> Result<Value, ShellError> {
        match self.anchors.get(&anchor_id) {
            Some(value) => Ok(value.clone()),
            None => todo!(),
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
    ctx.unhandled_tags(tag)?;

    todo!()
}

// gets called on Event::SequenceStart, returns on Event::SequenceEnd
// returns Value::List
fn parse_sequence<'i>(
    ctx: &mut ParseCtx<'i>,
    structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    ctx.unhandled_tags(tag)?;

    let mut values = Vec::new();
    loop {
        let event = ctx.next_event()?;
        match event {
            Event::Nothing | Event::Comment(..) => continue,
            Event::Alias(anchor_id) => values.push(ctx.get_anchor(ctx.alias(anchor_id)?)?),
            Event::Scalar(value, scalar_style, anchor_id, tag) => {
                let value = parse_scalar(ctx, value, scalar_style, tag)?;
                NonZeroUsize::new(anchor_id)
                    .map(|anchor_id| ctx.set_anchor(anchor_id, value.clone()));
                values.push(value);
            }
            Event::SequenceStart(structure_style, anchor_id, tag) => {
                let value = parse_sequence(ctx, structure_style, tag)?;
                NonZeroUsize::new(anchor_id)
                    .map(|anchor_id| ctx.set_anchor(anchor_id, value.clone()));
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
    structure_style: StructureStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<Value, ShellError> {
    ctx.unhandled_tags(tag)?;

    let mut values = HashMap::<String, Value>::new();
    loop {
        let key = 'key: loop {
            // expect a key or end
            let event = ctx.next_event()?;
            match event {
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
            let event = ctx.next_event()?;
            match event {
                Event::Nothing | Event::Comment(..) => continue,
                Event::Alias(anchor_id) => break 'value ctx.get_anchor(ctx.alias(anchor_id)?)?,
                Event::Scalar(value, scalar_style, anchor_id, tag) => {
                    let value = parse_scalar(ctx, value, scalar_style, tag)?;
                    NonZeroUsize::new(anchor_id)
                        .map(|anchor_id| ctx.set_anchor(anchor_id, value.clone()));
                    break 'value value;
                }
                Event::SequenceStart(structure_style, anchor_id, tag) => {
                    let value = parse_sequence(ctx, structure_style, tag)?;
                    NonZeroUsize::new(anchor_id)
                        .map(|anchor_id| ctx.set_anchor(anchor_id, value.clone()));
                    break 'value value;
                }
                Event::MappingStart(structure_style, anchor_id, tag) => {
                    let value = parse_mapping(ctx, structure_style, tag)?;
                    NonZeroUsize::new(anchor_id)
                        .map(|anchor_id| ctx.set_anchor(anchor_id, value.clone()));
                    break 'value value;
                }
                event => return Err(ctx.unexpected_event(event)),
            }
        };

        if let Some(duplicate) = values.insert(key, value) {
            todo!("throw duplicate error")
        }
    }
}

fn parse_key<'i>(
    ctx: &mut ParseCtx<'i>,
    value: Cow<'i, str>,
    scalar_style: ScalarStyle,
    tag: Option<Cow<'i, Tag>>,
) -> Result<String, ShellError> {
    todo!()
}
