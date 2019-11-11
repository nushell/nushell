use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub struct FromCSV;

#[derive(Deserialize)]
pub struct FromCSVArgs {
    headerless: bool,
    separator: Option<Tagged<Value>>,
}

impl WholeStreamCommand for FromCSV {
    fn name(&self) -> &str {
        "from-csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-csv")
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
            )
            .switch("headerless", "don't treat the first row as column names")
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table."
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
    separator: char,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!headerless)
        .delimiter(separator as u8)
        .from_reader(s.as_bytes());
    let tag = tag.into();

    let headers = if headerless {
        (1..=reader.headers()?.len())
            .map(|i| format!("Column{}", i))
            .collect::<Vec<String>>()
    } else {
        reader.headers()?.iter().map(String::from).collect()
    };

    let mut rows = vec![];
    for row in reader.records() {
        let mut tagged_row = TaggedDictBuilder::new(&tag);
        for (value, header) in row?.iter().zip(headers.iter()) {
            tagged_row.insert_tagged(
                header,
                Value::Primitive(Primitive::String(String::from(value))).tagged(&tag),
            )
        }
        rows.push(tagged_row.into_tagged_value());
    }

    Ok(Value::Table(rows).tagged(&tag))
}

fn from_csv(
    FromCSVArgs {
        headerless,
        separator,
    }: FromCSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;
    let sep = match separator {
        Some(Tagged {
            item: Value::Primitive(Primitive::String(s)),
            tag,
            ..
        }) => {
            let vec_s: Vec<char> = s.chars().collect();
            if vec_s.len() != 1 {
                return Err(ShellError::labeled_error(
                    "Expected a single separator char from --separator",
                    "requires a single character string input",
                    tag,
                ));
            };
            vec_s[0]
        }
        _ => ',',
    };

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

        match from_csv_string_to_value(concat_string, headerless, sep, name_tag.clone()) {
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
