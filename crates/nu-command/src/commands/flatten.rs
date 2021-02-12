use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    Dictionary, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "flatten"
    }

    fn signature(&self) -> Signature {
        Signature::build("flatten").rest(SyntaxShape::String, "optionally flatten data by column")
    }

    fn usage(&self) -> &str {
        "Flatten the table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        flatten(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "flatten a table",
                example: "echo [[N, u, s, h, e, l, l]] | flatten | first",
                result: Some(vec![Value::from("N")]),
            },
            Example {
                description: "flatten a column having a nested table",
                example: "echo [[origin, people]; [Ecuador, $(echo [[name, meal]; ['Andres', 'arepa']])]] | flatten | get meal",
                result: Some(vec![Value::from("arepa")]),
            },
            Example {
                description: "restrict the flattening by passing column names",
                example: "echo [[origin, crate, versions]; [World, $(echo [[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions | last | get versions",
                result: Some(vec![Value::from("0.22")]),
            }
        ]
    }
}

async fn flatten(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let (Arguments { rest: columns }, input) = args.process().await?;

    Ok(input
        .map(move |item| futures::stream::iter(flat_value(&columns, &item, &tag).into_iter()))
        .flatten()
        .to_output_stream())
}

enum TableInside<'a> {
    Entries(&'a str, &'a Tag, Vec<&'a Value>),
}

fn flat_value(
    columns: &[Tagged<String>],
    item: &Value,
    name_tag: impl Into<Tag>,
) -> Vec<Result<ReturnSuccess, ShellError>> {
    let tag = item.tag.clone();
    let name_tag = name_tag.into();

    let res = {
        if item.is_row() {
            let mut out = TaggedDictBuilder::new(tag);
            let mut a_table = None;
            let mut tables_explicitly_flattened = 0;

            for (column, value) in item.row_entries() {
                let column_requested = columns.iter().find(|c| c.item == *column);

                if let Value {
                    value: UntaggedValue::Row(Dictionary { entries: mapa }),
                    ..
                } = value
                {
                    if column_requested.is_none() && !columns.is_empty() {
                        if out.contains_key(&column) {
                            out.insert_value(format!("{}_{}", column, column), value.clone());
                        } else {
                            out.insert_value(column, value.clone());
                        }
                        continue;
                    }

                    for (k, v) in mapa.into_iter() {
                        if out.contains_key(k) {
                            out.insert_value(format!("{}_{}", column, k), v.clone());
                        } else {
                            out.insert_value(k, v.clone());
                        }
                    }
                } else if value.is_table() {
                    if tables_explicitly_flattened >= 1 && column_requested.is_some() {
                        let attempted = if let Some(name) = column_requested {
                            name.span()
                        } else {
                            name_tag.span
                        };

                        let already_flattened =
                            if let Some(TableInside::Entries(_, column_tag, _)) = a_table {
                                column_tag.span
                            } else {
                                name_tag.span
                            };

                        return vec![ReturnSuccess::value(
                            UntaggedValue::Error(ShellError::labeled_error_with_secondary(
                                "can only flatten one inner table at the same time",
                                "tried flattening more than one column with inner tables",
                                attempted,
                                "...but is flattened already",
                                already_flattened,
                            ))
                            .into_value(name_tag),
                        )];
                    }

                    if !columns.is_empty() {
                        if let Some(requested) = column_requested {
                            a_table = Some(TableInside::Entries(
                                &requested.item,
                                &requested.tag,
                                value.table_entries().collect(),
                            ));

                            tables_explicitly_flattened += 1;
                        } else {
                            out.insert_value(column, value.clone());
                        }
                    } else if a_table.is_none() {
                        a_table = Some(TableInside::Entries(
                            &column,
                            &value.tag,
                            value.table_entries().collect(),
                        ))
                    } else {
                        out.insert_value(column, value.clone());
                    }
                } else {
                    out.insert_value(column, value.clone());
                }
            }

            let mut expanded = vec![];

            if let Some(TableInside::Entries(column, _, entries)) = a_table {
                for entry in entries.into_iter() {
                    let mut base = out.clone();
                    base.insert_value(column, entry.clone());
                    expanded.push(base.into_value());
                }
            } else {
                expanded.push(out.into_value());
            }

            expanded
        } else if item.is_table() {
            item.table_entries().map(Clone::clone).collect()
        } else {
            vec![item.clone()]
        }
    };

    res.into_iter().map(ReturnSuccess::value).collect()
}
