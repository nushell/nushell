use crate::commands::WholeStreamCommand;
use crate::errors::ranged;
use crate::object::{Primitive, Value};
use crate::prelude::*;

pub struct ToTOML;

impl WholeStreamCommand for ToTOML {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_toml(args, registry)
    }

    fn name(&self) -> &str {
        "to-toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-toml")
    }
}

pub fn value_to_toml_value(v: &Value) -> Result<toml::Value, ShellError> {
    Ok(match v {
        Value::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            toml::Value::Integer(ranged(b.to_i64(), "i64", b.tagged_unknown())?)
        }
        Value::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        Value::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        Value::Primitive(Primitive::Decimal(f)) => {
            toml::Value::Float(ranged(f.to_f64(), "f64", f.tagged_unknown())?)
        }
        Value::Primitive(Primitive::Int(i)) => toml::Value::Integer(*i),
        Value::Primitive(Primitive::Nothing) => toml::Value::String("<Nothing>".to_string()),
        Value::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => toml::Value::String(s.display().to_string()),

        Value::List(l) => toml::Value::Array(collect_values(l)?),
        Value::Block(_) => toml::Value::String("<Block>".to_string()),
        Value::Binary(b) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        Value::Object(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_toml_value(v)?);
            }
            toml::Value::Table(m)
        }
    })
}

fn collect_values(input: &Vec<Tagged<Value>>) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_toml_value(value)?);
    }

    Ok(out)
}

fn to_toml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_span = args.name_span();
    let out = args.input;

    Ok(out
        .values
        .map(move |a| match toml::to_string(&value_to_toml_value(&a)?) {
            Ok(val) => {
                return ReturnSuccess::value(
                    Value::Primitive(Primitive::String(val)).simple_spanned(name_span),
                )
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected an object with TOML-compatible structure from pipeline",
                "requires TOML-compatible input",
                name_span,
                format!("{} originates from here", a.item.type_name()),
                a.span(),
            )),
        })
        .to_output_stream())
}
