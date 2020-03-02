use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromYAML;

impl WholeStreamCommand for FromYAML {
    fn name(&self) -> &str {
        "from-yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-yaml")
    }

    fn usage(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_yaml(args, registry)
    }
}

pub struct FromYML;

impl WholeStreamCommand for FromYML {
    fn name(&self) -> &str {
        "from-yml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-yml")
    }

    fn usage(&self) -> &str {
        "Parse text as .yaml/.yml and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_yaml(args, registry)
    }
}

fn convert_yaml_value_to_nu_value(
    v: &serde_yaml::Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    Ok(match v {
        serde_yaml::Value::Bool(b) => UntaggedValue::boolean(*b).into_value(tag),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            UntaggedValue::int(n.as_i64().ok_or_else(|| {
                ShellError::labeled_error(
                    "Expected a compatible number",
                    "expected a compatible number",
                    &tag,
                )
            })?)
            .into_value(tag)
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            UntaggedValue::decimal(n.as_f64().ok_or_else(|| {
                ShellError::labeled_error(
                    "Expected a compatible number",
                    "expected a compatible number",
                    &tag,
                )
            })?)
            .into_value(tag)
        }
        serde_yaml::Value::String(s) => UntaggedValue::string(s).into_value(tag),
        serde_yaml::Value::Sequence(a) => {
            let result: Result<Vec<Value>, ShellError> = a
                .iter()
                .map(|x| convert_yaml_value_to_nu_value(x, &tag))
                .collect();
            UntaggedValue::Table(result?).into_value(tag)
        }
        serde_yaml::Value::Mapping(t) => {
            let mut collected = TaggedDictBuilder::new(&tag);

            for (k, v) in t.iter() {
                match k {
                    serde_yaml::Value::String(k) => {
                        collected.insert_value(k.clone(), convert_yaml_value_to_nu_value(v, &tag)?);
                    }
                    _ => unimplemented!("Unknown key type"),
                }
            }

            collected.into_value()
        }
        serde_yaml::Value::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(tag),
        x => unimplemented!("Unsupported yaml case: {:?}", x),
    })
}

pub fn from_yaml_string_to_value(s: String, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).map_err(|x| {
        ShellError::labeled_error(
            format!("Could not load yaml: {}", x),
            "could not load yaml from text",
            &tag,
        )
    })?;
    Ok(convert_yaml_value_to_nu_value(&v, tag)?)
}

fn from_yaml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let name_span = tag.span;
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            latest_tag = Some(value.tag.clone());
            let value_span = value.tag.span;

            if let Ok(s) = value.as_string() {
                concat_string.push_str(&s);
            }
            else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))
            }
        }

        match from_yaml_string_to_value(concat_string, tag.clone()) {
            Ok(x) => match x {
                Value { value: UntaggedValue::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as YAML",
                    "input cannot be parsed as YAML",
                    &tag,
                    "value originates from here",
                    &last_tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
