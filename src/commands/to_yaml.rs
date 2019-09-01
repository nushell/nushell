use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, Value};
use crate::prelude::*;

pub struct ToYAML;

impl WholeStreamCommand for ToYAML {
    fn name(&self) -> &str {
        "to-yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-yaml")
    }

    fn usage(&self) -> &str {
        "Convert table into .yaml/.yml text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_yaml(args, registry)
    }
}

pub fn value_to_yaml_value(v: &Tagged<Value>) -> Result<serde_yaml::Value, ShellError> {
    Ok(match v.item() {
        Value::Primitive(Primitive::Boolean(b)) => serde_yaml::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(b.to_f64().unwrap()))
        }
        Value::Primitive(Primitive::Date(d)) => serde_yaml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::BeginningOfStream) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::Decimal(f)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(f.to_f64().unwrap()))
        }
        Value::Primitive(Primitive::Int(i)) => serde_yaml::Value::Number(serde_yaml::Number::from(
            CoerceInto::<i64>::coerce_into(i.tagged(v.tag), "converting to YAML number")?,
        )),
        Value::Primitive(Primitive::Nothing) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::String(s)) => serde_yaml::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => serde_yaml::Value::String(s.display().to_string()),

        Value::List(l) => {
            let mut out = vec![];

            for value in l {
                out.push(value_to_yaml_value(value)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Block(_) => serde_yaml::Value::Null,
        Value::Binary(b) => serde_yaml::Value::Sequence(
            b.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        Value::Object(o) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in o.entries.iter() {
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml_value(v)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
    })
}

fn to_yaml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_span = args.name_span();
    let out = args.input;
    Ok(out
        .values
        .map(
            move |a| match serde_yaml::to_string(&value_to_yaml_value(&a)?) {
                Ok(x) => ReturnSuccess::value(
                    Value::Primitive(Primitive::String(x)).simple_spanned(name_span),
                ),
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Expected an object with YAML-compatible structure from pipeline",
                    "requires YAML-compatible input",
                    name_span,
                    format!("{} originates from here", a.item.type_name()),
                    a.span(),
                )),
            },
        )
        .to_output_stream())
}
