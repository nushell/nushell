use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub struct FromCSV;

#[derive(Deserialize)]
pub struct FromCSVArgs {
    headerless: bool,
}

impl WholeStreamCommand for FromCSV {
    fn name(&self) -> &str {
        "from-csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-csv").switch("headerless")
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_csv)?.run()
    }
}

pub fn from_csv_string_to_value(
    s: String,
    headerless: bool,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(s.as_bytes());
    let tag = tag.into();

    let mut fields: VecDeque<String> = VecDeque::new();
    let mut iter = reader.records();
    let mut rows = vec![];

    if let Some(result) = iter.next() {
        let line = result?;

        for (idx, item) in line.iter().enumerate() {
            if headerless {
                fields.push_back(format!("Column{}", idx + 1));
            } else {
                fields.push_back(item.to_string());
            }
        }
    }

    loop {
        if let Some(row_values) = iter.next() {
            let row_values = row_values?;

            let mut row = TaggedDictBuilder::new(tag.clone());

            for (idx, entry) in row_values.iter().enumerate() {
                row.insert_tagged(
                    fields.get(idx).unwrap(),
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(&tag),
                );
            }

            rows.push(row.into_tagged_value());
        } else {
            break;
        }
    }

    Ok(Value::Table(rows).tagged(&tag))
}

fn from_csv(
    FromCSVArgs {
        headerless: skip_headers,
    }: FromCSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;

    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag.clone());
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_tag.clone(),
                    "value originates from here",
                    value_tag.clone(),
                )),

            }
        }

        match from_csv_string_to_value(concat_string, skip_headers, name_tag.clone()) {
            Ok(x) => match x {
                Tagged { item: Value::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as CSV",
                    "input cannot be parsed as CSV",
                    name_tag.clone(),
                    "value originates from here",
                    last_tag.clone(),
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
