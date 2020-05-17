use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UnspannedPathMember, UntaggedValue, Value};
use serde::Serialize;
use serde_json::json;

pub struct ToJSON;

#[derive(Deserialize)]
pub struct ToJSONArgs {
    pretty: Option<Value>,
}

impl WholeStreamCommand for ToJSON {
    fn name(&self) -> &str {
        "to json"
    }

    fn signature(&self) -> Signature {
        Signature::build("to json").switch("pretty", "Formats the json text", Some('p'))
    }

    fn usage(&self) -> &str {
        "Converts table data into json text."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_json(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description:
                    "Outputs an unformatted JSON string representing the contents of this table",
                example: "to json",
            },
            Example {
                description:
                    "Outputs a formatted JSON string representing the contents of this table",
                example: "to json --pretty",
            },
        ]
    }
}

pub fn value_to_json_value(v: &Value) -> Result<serde_json::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        UntaggedValue::Primitive(Primitive::Bytes(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("What about really big numbers")),
        ),
        UntaggedValue::Primitive(Primitive::Duration(secs)) => {
            serde_json::Value::Number(serde_json::Number::from(*secs))
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            if let Some(f) = f.to_f64() {
                if let Some(num) = serde_json::Number::from_f64(
                    f.to_f64().expect("TODO: What about really big decimals?"),
                ) {
                    serde_json::Value::Number(num)
                } else {
                    return Err(ShellError::labeled_error(
                        "Could not convert value to decimal number",
                        "could not convert to decimal",
                        &v.tag,
                    ));
                }
            } else {
                return Err(ShellError::labeled_error(
                    "Could not convert value to decimal number",
                    "could not convert to decimal",
                    &v.tag,
                ));
            }
        }

        UntaggedValue::Primitive(Primitive::Int(i)) => {
            serde_json::Value::Number(serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                i.tagged(&v.tag),
                "converting to JSON number",
            )?))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Pattern(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::Line(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => serde_json::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => {
                        Ok(serde_json::Value::String(string.clone()))
                    }
                    UnspannedPathMember::Int(int) => Ok(serde_json::Value::Number(
                        serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                            int.tagged(&v.tag),
                            "converting to JSON number",
                        )?),
                    )),
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        UntaggedValue::Primitive(Primitive::Path(s)) => {
            serde_json::Value::String(s.display().to_string())
        }

        UntaggedValue::Table(l) => serde_json::Value::Array(json_list(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => {
            serde_json::Value::Null
        }
        UntaggedValue::Primitive(Primitive::Binary(b)) => serde_json::Value::Array(
            b.iter()
                .map(|x| {
                    serde_json::Number::from_f64(*x as f64).ok_or_else(|| {
                        ShellError::labeled_error(
                            "Can not convert number from floating point",
                            "can not convert to number",
                            &v.tag,
                        )
                    })
                })
                .collect::<Result<Vec<serde_json::Number>, ShellError>>()?
                .into_iter()
                .map(serde_json::Value::Number)
                .collect(),
        ),
        UntaggedValue::Row(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            serde_json::Value::Object(m)
        }
    })
}

fn json_list(input: &[Value]) -> Result<Vec<serde_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn to_json(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (ToJSONArgs { pretty }, mut input) = args.process(&registry).await?;
        let args = args.evaluate_once(&registry).await?;
        let name_tag = args.name_tag();
        let name_span = name_tag.span;
        let input: Vec<Value> = input.collect().await;

        let to_process_input = if input.len() > 1 {
            let tag = input[0].tag.clone();
            vec![Value { value: UntaggedValue::Table(input), tag } ]
        } else if input.len() == 1 {
            input
        } else {
            vec![]
        };

        for value in to_process_input {
            match value_to_json_value(&value) {
                Ok(json_value) => {
                    let value_span = value.tag.span;

                    match serde_json::to_string(&json_value) {
                        Ok(mut serde_json_string) => {
                            if let Some(pretty_value) = pretty {
                                let mut pretty_format_failed = true;

                                if let Ok(serde_json_value) = serde_json::from_str::<serde_json::Value>(serde_json_string.as_str()) {
                                    let indentation_string = std::iter::repeat(" ").take(4).collect::<String>();
                                    let serde_formatter = serde_json::ser::PrettyFormatter::with_indent(indentation_string.as_bytes());
                                    let serde_buffer = Vec::new();
                                    let mut serde_serializer = serde_json::Serializer::with_formatter(serde_buffer, serde_formatter);
                                    let serde_json_object = json!(serde_json_value);

                                    if let Ok(()) = serde_json_object.serialize(&mut serde_serializer) {
                                        if let Ok(ser_json_string) = String::from_utf8(serde_serializer.into_inner()) {
                                            pretty_format_failed = false;
                                            serde_json_string = ser_json_string
                                        }
                                    }
                                }

                                if pretty_format_failed {
                                    yield Err(ShellError::labeled_error("Pretty formatting failed", "failed", pretty_value.tag()));
                                    return;
                                }
                            }

                            yield ReturnSuccess::value(
                                UntaggedValue::Primitive(Primitive::String(serde_json_string)).into_value(&name_tag),
                            )
                        },
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with JSON-compatible structure.tag() from pipeline",
                            "requires JSON-compatible input",
                            name_span,
                            "originates from here".to_string(),
                            value_span,
                        )),
                    }
                }
                _ => yield Err(ShellError::labeled_error(
                    "Expected a table with JSON-compatible structure from pipeline",
                    "requires JSON-compatible input",
                    &name_tag))
            }
        }
    };

    Ok(stream.to_output_stream())
}

// Error handle failures

// Release:
// rustfmt (through editor and try manually)
// clippy
// Check diff and make sure strings for docs all look good / match

// Later:
// Add range for tabs (1 - 8)
