use crate::prelude::*;
use nu_errors::ShellError;
use nu_parser::hir::syntax_shape::{ExpandContext, SignatureRegistry};
use nu_parser::utils::{parse_line_with_separator as parse, LineSeparatedShape};
use nu_parser::TokensIterator;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::nom_input;

use derive_new::new;

pub fn from_delimited_data(
    headerless: bool,
    sep: char,
    format_name: &'static str,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;

    let stream = async_stream! {
        let concat_string = input.collect_string(name_tag.clone()).await?;

        match from_delimited_string_to_value(concat_string.item, headerless, sep, name_tag.clone()) {
            Ok(rows) => {
                for row in rows {
                    match row {
                        Value { value: UntaggedValue::Table(list), .. } => {
                            for l in list {
                                yield ReturnSuccess::value(l);
                            }
                        }
                        x => yield ReturnSuccess::value(x),
                    }
                }
            },
            Err(err) => {
                let line_one = format!("Could not parse as {}", format_name);
                let line_two = format!("input cannot be parsed as {}", format_name);
                yield Err(ShellError::labeled_error_with_secondary(
                    line_one,
                    line_two,
                    name_tag.clone(),
                    "value originates from here",
                    concat_string.tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}

#[derive(Debug, Clone, new)]
pub struct EmptyRegistry {
    #[new(default)]
    signatures: indexmap::IndexMap<String, Signature>,
}

impl EmptyRegistry {}

impl SignatureRegistry for EmptyRegistry {
    fn has(&self, _name: &str) -> bool {
        false
    }
    fn get(&self, _name: &str) -> Option<Signature> {
        None
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        Box::new(self.clone())
    }
}

fn from_delimited_string_to_value(
    s: String,
    headerless: bool,
    sep: char,
    tag: impl Into<Tag>,
) -> Result<Vec<Value>, ShellError> {
    let tag = tag.into();

    let mut entries = s.lines();

    let mut fields = vec![];
    let mut out = vec![];

    if let Some(first_entry) = entries.next() {
        let tokens = match parse(&sep.to_string(), nom_input(first_entry)) {
            Ok((_, tokens)) => tokens,
            Err(err) => return Err(ShellError::parse_error(err)),
        };

        let tokens_span = tokens.span;
        let source: nu_source::Text = tokens_span.slice(&first_entry).into();

        if !headerless {
            fields = tokens
                .item
                .iter()
                .filter(|token| !token.is_separator())
                .map(|field| field.source(&source).to_string())
                .collect::<Vec<_>>();
        }

        let registry = Box::new(EmptyRegistry::new());
        let ctx = ExpandContext::new(registry, &source, None);

        let mut iterator = TokensIterator::new(&tokens.item, ctx, tokens_span);
        let (results, tokens_identified) = iterator.expand(LineSeparatedShape);
        let results = results?;

        let mut row = TaggedDictBuilder::new(&tag);

        if headerless {
            let fallback_columns = (1..=tokens_identified)
                .map(|i| format!("Column{}", i))
                .collect::<Vec<String>>();

            for (idx, field) in results.into_iter().enumerate() {
                let key = if headerless {
                    &fallback_columns[idx]
                } else {
                    &fields[idx]
                };

                row.insert_value(key, field.into_value(&tag));
            }

            out.push(row.into_value())
        }
    }

    for entry in entries {
        let tokens = match parse(&sep.to_string(), nom_input(entry)) {
            Ok((_, tokens)) => tokens,
            Err(err) => return Err(ShellError::parse_error(err)),
        };
        let tokens_span = tokens.span;

        let source: nu_source::Text = tokens_span.slice(&entry).into();
        let registry = Box::new(EmptyRegistry::new());
        let ctx = ExpandContext::new(registry, &source, None);

        let mut iterator = TokensIterator::new(&tokens.item, ctx, tokens_span);
        let (results, tokens_identified) = iterator.expand(LineSeparatedShape);
        let results = results?;

        let mut row = TaggedDictBuilder::new(&tag);

        let fallback_columns = (1..=tokens_identified)
            .map(|i| format!("Column{}", i))
            .collect::<Vec<String>>();

        for (idx, field) in results.into_iter().enumerate() {
            let key = if headerless {
                &fallback_columns[idx]
            } else {
                match fields.get(idx) {
                    Some(key) => key,
                    None => &fallback_columns[idx],
                }
            };

            row.insert_value(key, field.into_value(&tag));
        }

        out.push(row.into_value())
    }

    Ok(out)
}
