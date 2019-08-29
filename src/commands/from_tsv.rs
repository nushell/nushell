use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, TaggedDictBuilder, Value};
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
        Signature::build("from-tsv").switch("headerless")
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
        .has_headers(false)
        .delimiter(b'\t')
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

            let mut row = TaggedDictBuilder::new(tag);

            for (idx, entry) in row_values.iter().enumerate() {
                row.insert_tagged(
                    fields.get(idx).unwrap(),
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(tag),
                );
            }

            rows.push(row.into_tagged_value());
        } else {
            break;
        }
    }

    Ok(Tagged::from_item(Value::List(rows), tag))
}

fn from_tsv(
    FromTSVArgs {
        headerless: skip_headers,
    }: FromTSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_span = name;

    let stream = async_stream_block! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag);
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_tag.span,
                )),

            }
        }

        match from_tsv_string_to_value(concat_string, skip_headers, name_span) {
            Ok(x) => match x {
                Tagged { item: Value::List(list), .. } => {
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
                    name_span,
                    "value originates from here",
                    last_tag.span,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
