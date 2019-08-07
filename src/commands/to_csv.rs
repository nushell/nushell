use crate::object::{Primitive, Value};
use crate::prelude::*;
use csv::WriterBuilder;

pub fn value_to_csv_value(v: &Value) -> Value {
    match v {
        Value::Primitive(Primitive::String(s)) => Value::Primitive(Primitive::String(s.clone())),
        Value::Primitive(Primitive::Nothing) => Value::Primitive(Primitive::Nothing),
        Value::Object(o) => Value::Object(o.clone()),
        Value::List(l) => Value::List(l.clone()),
        Value::Block(_) => Value::Primitive(Primitive::Nothing),
        _ => Value::Primitive(Primitive::Nothing),
    }
}

pub fn to_string(v: &Value) -> Result<String, Box<dyn std::error::Error>> {
    match v {
        Value::List(_l) => return Ok(String::from("[list list]")),
        Value::Object(o) => {
            let mut wtr = WriterBuilder::new().from_writer(vec![]);
            let mut fields: VecDeque<String> = VecDeque::new();
            let mut values: VecDeque<String> = VecDeque::new();

            for (k, v) in o.entries.iter() {
                fields.push_back(k.clone());
                values.push_back(to_string(&v)?);
            }

            wtr.write_record(fields).expect("can not write.");
            wtr.write_record(values).expect("can not write.");

            return Ok(String::from_utf8(wtr.into_inner()?)?);
        }
        Value::Primitive(Primitive::String(s)) => return Ok(s.to_string()),
        _ => return Err("Bad input".into()),
    }
}

pub fn to_csv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let name_span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| match to_string(&value_to_csv_value(&a.item)) {
            Ok(x) => ReturnSuccess::value(
                Value::Primitive(Primitive::String(x)).simple_spanned(name_span),
            ),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected an object with CSV-compatible structure from pipeline",
                "requires CSV-compatible input",
                name_span,
                format!("{} originates from here", a.item.type_name()),
                a.span(),
            )),
        })
        .to_output_stream())
}
