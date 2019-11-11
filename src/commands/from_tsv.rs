use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub struct FromTSV;

#[derive(Deserialize)]
pub struct FromTSVArgs {
    headerless: bool,
}

impl WholeStreamCommand for FromTSV {
    fn name(&self) -> &str {
        "from-tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-tsv")
            .switch("headerless", "don't treat the first row as column names")
    }

    fn usage(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_tsv)?.run()
    }
}

pub fn from_tsv_string_to_value(
    s: String,
    headerless: bool,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!headerless)
        .delimiter(b'\t')
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

fn from_tsv(
    FromTSVArgs { headerless }: FromTSVArgs,
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
                    &name_tag,
                    "value originates from here",
                    &value_tag,
                )),

            }
        }

        match from_tsv_string_to_value(concat_string, headerless, name_tag.clone()) {
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
                    "Could not parse as TSV",
                    "input cannot be parsed as TSV",
                    &name_tag,
                    "value originates from here",
                    &last_tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
