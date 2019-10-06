use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, Value};
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
        Signature::build("to-tsv").switch("headerless")
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

pub fn value_to_tsv_value(tagged_value: &Tagged<Value>) -> Tagged<Value> {
    let v = &tagged_value.item;

    match v {
        Value::Primitive(Primitive::String(s)) => Value::Primitive(Primitive::String(s.clone())),
        Value::Primitive(Primitive::Nothing) => Value::Primitive(Primitive::Nothing),
        Value::Primitive(Primitive::Boolean(b)) => Value::Primitive(Primitive::Boolean(b.clone())),
        Value::Primitive(Primitive::Decimal(f)) => Value::Primitive(Primitive::Decimal(f.clone())),
        Value::Primitive(Primitive::Int(i)) => Value::Primitive(Primitive::Int(i.clone())),
        Value::Primitive(Primitive::Path(x)) => Value::Primitive(Primitive::Path(x.clone())),
        Value::Primitive(Primitive::Bytes(b)) => Value::Primitive(Primitive::Bytes(b.clone())),
        Value::Primitive(Primitive::Date(d)) => Value::Primitive(Primitive::Date(d.clone())),
        Value::Row(o) => Value::Row(o.clone()),
        Value::Table(l) => Value::Table(l.clone()),
        Value::Block(_) => Value::Primitive(Primitive::Nothing),
        _ => Value::Primitive(Primitive::Nothing),
    }
    .tagged(tagged_value.tag)
}

fn to_string_helper(tagged_value: &Tagged<Value>) -> Result<String, ShellError> {
    let v = &tagged_value.item;
    match v {
        Value::Primitive(Primitive::Date(d)) => Ok(d.to_string()),
        Value::Primitive(Primitive::Bytes(b)) => Ok(format!("{}", b)),
        Value::Primitive(Primitive::Boolean(_)) => Ok(tagged_value.as_string()?),
        Value::Primitive(Primitive::Decimal(_)) => Ok(tagged_value.as_string()?),
        Value::Primitive(Primitive::Int(_)) => Ok(tagged_value.as_string()?),
        Value::Primitive(Primitive::Path(_)) => Ok(tagged_value.as_string()?),
        Value::Table(_) => return Ok(String::from("[table]")),
        Value::Row(_) => return Ok(String::from("[row]")),
        Value::Primitive(Primitive::String(s)) => return Ok(s.to_string()),
        _ => {
            return Err(ShellError::labeled_error(
                "Unexpected value",
                "original value",
                tagged_value.tag,
            ))
        }
    }
}

fn merge_descriptors(values: &[Tagged<Value>]) -> Vec<String> {
    let mut ret = vec![];
    for value in values {
        for desc in value.data_descriptors() {
            if !ret.contains(&desc) {
                ret.push(desc);
            }
        }
    }
    ret
}

pub fn to_string(tagged_value: &Tagged<Value>) -> Result<String, ShellError> {
    let v = &tagged_value.item;

    match v {
        Value::Row(o) => {
            let mut wtr = WriterBuilder::new().delimiter(b'\t').from_writer(vec![]);
            let mut fields: VecDeque<String> = VecDeque::new();
            let mut values: VecDeque<String> = VecDeque::new();

            for (k, v) in o.entries.iter() {
                fields.push_back(k.clone());
                values.push_back(to_string_helper(&v)?);
            }

            wtr.write_record(fields).expect("can not write.");
            wtr.write_record(values).expect("can not write.");

            return Ok(String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    tagged_value.tag,
                )
            })?)
            .map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    tagged_value.tag,
                )
            })?);
        }
        Value::Table(list) => {
            let mut wtr = WriterBuilder::new().delimiter(b'\t').from_writer(vec![]);

            let merged_descriptors = merge_descriptors(&list);
            wtr.write_record(&merged_descriptors)
                .expect("can not write.");

            for l in list {
                let mut row = vec![];
                for desc in &merged_descriptors {
                    match l.item.get_data_by_key(&desc) {
                        Some(s) => {
                            row.push(to_string_helper(s)?);
                        }
                        None => {
                            row.push(String::new());
                        }
                    }
                }
                wtr.write_record(&row).expect("can not write");
            }

            return Ok(String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    tagged_value.tag,
                )
            })?)
            .map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    tagged_value.tag,
                )
            })?);
        }
        _ => return to_string_helper(tagged_value),
    }
}

fn to_tsv(
    ToTSVArgs { headerless }: ToTSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;
    let stream = async_stream! {
         let input: Vec<Tagged<Value>> = input.values.collect().await;

         let to_process_input = if input.len() > 1 {
             let tag = input[0].tag;
             vec![Tagged { item: Value::Table(input), tag } ]
         } else if input.len() == 1 {
             input
         } else {
             vec![]
         };

         for value in to_process_input {
             match to_string(&value_to_tsv_value(&value)) {
                 Ok(x) => {
                     let converted = if headerless {
                         x.lines().skip(1).collect()
                     } else {
                         x
                     };
                     yield ReturnSuccess::value(Value::Primitive(Primitive::String(converted)).tagged(name_tag))
                 }
                 _ => {
                     yield Err(ShellError::labeled_error_with_secondary(
                         "Expected a table with TSV-compatible structure.tag() from pipeline",
                         "requires TSV-compatible input",
                         name_tag,
                         "originates from here".to_string(),
                         value.tag(),
                     ))
                 }
             }
         }
    };

    Ok(stream.to_output_stream())
}
