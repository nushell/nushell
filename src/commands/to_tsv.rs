use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use csv::WriterBuilder;

pub struct ToTSV;

#[derive(Deserialize)]
pub struct ToTSVArgs {
    headerless: bool,
}

impl WholeStreamCommand for ToTSV {
    fn name(&self) -> &str {
        "to-tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-tsv")
            .switch("headerless")
    }

    fn usage(&self) -> &str {
        "Convert table into .tsv text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, to_tsv)?.run()
    }
}

pub fn value_to_tsv_value(v: &Value) -> Value {
    match v {
        Value::Primitive(Primitive::String(s)) => Value::Primitive(Primitive::String(s.clone())),
        Value::Primitive(Primitive::Nothing) => Value::Primitive(Primitive::Nothing),
        Value::Primitive(Primitive::Boolean(b)) => Value::Primitive(Primitive::Boolean(b.clone())),
        Value::Primitive(Primitive::Bytes(b)) => Value::Primitive(Primitive::Bytes(b.clone())),
        Value::Primitive(Primitive::Date(d)) => Value::Primitive(Primitive::Date(d.clone())),
        Value::Object(o) => Value::Object(o.clone()),
        Value::List(l) => Value::List(l.clone()),
        Value::Block(_) => Value::Primitive(Primitive::Nothing),
        _ => Value::Primitive(Primitive::Nothing),
    }
}

fn to_string_helper(v: &Value) -> Result<String, Box<dyn std::error::Error>> {
    match v {
        Value::Primitive(Primitive::Date(d)) => Ok(d.to_string()),
        Value::Primitive(Primitive::Bytes(b)) => Ok(format!("{}", b)),
        Value::Primitive(Primitive::Boolean(_)) => Ok(v.as_string()?),
        Value::List(_) => return Ok(String::from("[list list]")),
        Value::Object(_) => return Ok(String::from("[object]")),
        Value::Primitive(Primitive::String(s)) => return Ok(s.to_string()),
        _ => return Err("Bad input".into()),
    }
}

pub fn to_string(v: &Value) -> Result<String, Box<dyn std::error::Error>> {
    match v {
        Value::Object(o) => {
            let mut wtr = WriterBuilder::new().delimiter(b'\t').from_writer(vec![]);
            let mut fields: VecDeque<String> = VecDeque::new();
            let mut values: VecDeque<String> = VecDeque::new();

            for (k, v) in o.entries.iter() {
                fields.push_back(k.clone());
                values.push_back(to_string_helper(&v)?);
            }

            wtr.write_record(fields).expect("can not write.");
            wtr.write_record(values).expect("can not write.");

            return Ok(String::from_utf8(wtr.into_inner()?)?);
        }
        _ => return to_string_helper(&v),
    }
}

fn to_tsv(
    ToTSVArgs { headerless }: ToTSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_span = name;
    let out = input;

    Ok(out
        .values
        .map(move |a| match to_string(&value_to_tsv_value(&a.item)) {
            Ok(x) => {
                let converted = if headerless {
                    x.lines().skip(1).collect()
                } else {
                    x
                };

                ReturnSuccess::value(
                    Value::Primitive(Primitive::String(converted)).simple_spanned(name_span),
                )
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected an object with TSV-compatible structure from pipeline",
                "requires TSV-compatible input",
                name_span,
                format!("{} originates from here", a.item.type_name()),
                a.span(),
            )),
        })
        .to_output_stream())
}
