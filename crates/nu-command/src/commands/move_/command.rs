use crate::prelude::*;
use nu_data::base::select_fields;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::HasFallibleSpan;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    rest: Vec<ColumnPath>,
    after: Option<ColumnPath>,
    before: Option<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "move"
    }

    fn signature(&self) -> Signature {
        Signature::build("move")
            .rest(SyntaxShape::ColumnPath, "the columns to move")
            .named(
                "after",
                SyntaxShape::ColumnPath,
                "the column that will precede the columns moved",
                None,
            )
            .named(
                "before",
                SyntaxShape::ColumnPath,
                "the column that will be next the columns moved",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Move columns."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        use nu_test_support::value::*;

        vec![
            Example {
                description: "Move the column \"type\" before the column \"name\"",
                example: r#"ls | move type --before name | first"#,
                result: Some(vec![row! {
                    "type".into() =>         string("File"),
                    "name".into() =>   string("Andres.txt"),
                    "chickens".into() =>            int(10),
                    "modified".into() => date("2019-07-23")
                }]),
            },
            Example {
                description: "or move the column \"chickens\" after \"name\"",
                example: r#"ls | move chickens --after name | first"#,
                result: Some(vec![row! {
                    "name".into() =>   string("Andres.txt"),
                    "chickens".into() =>            int(10),
                    "type".into() =>         string("File"),
                    "modified".into() => date("2019-07-23")
                }]),
            },
            Example {
                description: "you can selectively move many columns in either direction",
                example: r#"ls | move name chickens --after type | first"#,
                result: Some(vec![row! {
                    "type".into() =>         string("File"),
                    "name".into() =>   string("Andres.txt"),
                    "chickens".into() =>            int(10),
                    "modified".into() => date("2019-07-23")
                }]),
            },
        ]
    }
}

async fn operate(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = raw_args.call_info.name_tag.clone();
    let (
        Arguments {
            rest: mut columns,
            before,
            after,
        },
        input,
    ) = raw_args.process().await?;

    if columns.is_empty() {
        return Err(ShellError::labeled_error(
            "expected columns",
            "expected columns",
            name,
        ));
    }

    if columns.iter().any(|c| c.members().len() > 1) {
        return Err(ShellError::labeled_error(
            "expected columns",
            "expected columns",
            name,
        ));
    }

    if vec![&after, &before]
        .iter()
        .map(|o| if o.is_some() { 1 } else { 0 })
        .sum::<usize>()
        > 1
    {
        return Err(ShellError::labeled_error(
            "can't move column(s)",
            "pick exactly one (before, after)",
            name,
        ));
    }

    if let Some(after) = after {
        let member = columns.remove(0);

        Ok(input
            .map(move |item| {
                let member = vec![member.clone()];
                let column_paths = vec![&member, &columns]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<&ColumnPath>>();

                let after_span = after.maybe_span().unwrap_or_else(Span::unknown);

                if after.members().len() == 1 {
                    let keys = column_paths
                        .iter()
                        .filter_map(|c| c.last())
                        .map(|c| c.as_string())
                        .collect::<Vec<_>>();

                    if let Some(column) = after.last() {
                        if !keys.contains(&column.as_string()) {
                            ReturnSuccess::value(move_after(&item, &keys, &after)?)
                        } else {
                            let msg =
                                format!("can't move column {} after itself", column.as_string());
                            Err(ShellError::labeled_error(
                                "can't move column",
                                msg,
                                after_span,
                            ))
                        }
                    } else {
                        Err(ShellError::labeled_error(
                            "expected column",
                            "expected column",
                            after_span,
                        ))
                    }
                } else {
                    Err(ShellError::labeled_error(
                        "expected column",
                        "expected column",
                        after_span,
                    ))
                }
            })
            .to_output_stream())
    } else if let Some(before) = before {
        let member = columns.remove(0);

        Ok(input
            .map(move |item| {
                let member = vec![member.clone()];
                let column_paths = vec![&member, &columns]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<&ColumnPath>>();

                let before_span = before.maybe_span().unwrap_or_else(Span::unknown);

                if before.members().len() == 1 {
                    let keys = column_paths
                        .iter()
                        .filter_map(|c| c.last())
                        .map(|c| c.as_string())
                        .collect::<Vec<_>>();

                    if let Some(column) = before.last() {
                        if !keys.contains(&column.as_string()) {
                            ReturnSuccess::value(move_before(&item, &keys, &before)?)
                        } else {
                            let msg =
                                format!("can't move column {} before itself", column.as_string());
                            Err(ShellError::labeled_error(
                                "can't move column",
                                msg,
                                before_span,
                            ))
                        }
                    } else {
                        Err(ShellError::labeled_error(
                            "expected column",
                            "expected column",
                            before_span,
                        ))
                    }
                } else {
                    Err(ShellError::labeled_error(
                        "expected column",
                        "expected column",
                        before_span,
                    ))
                }
            })
            .to_output_stream())
    } else {
        Err(ShellError::labeled_error(
            "no columns given",
            "no columns given",
            name,
        ))
    }
}

fn move_after(table: &Value, columns: &[String], from: &ColumnPath) -> Result<Value, ShellError> {
    let from_fields = from.maybe_span().unwrap_or_else(Span::unknown);
    let from = if let Some((last, _)) = from.split_last() {
        last.as_string()
    } else {
        return Err(ShellError::labeled_error(
            "unknown column",
            "unknown column",
            from_fields,
        ));
    };

    let columns_moved = table.data_descriptors().into_iter().map(|name| {
        if columns.contains(&name) {
            None
        } else {
            Some(name)
        }
    });

    let mut reordered_columns = vec![];
    let mut insert = false;
    let mut inserted = false;

    for name in columns_moved.into_iter() {
        if let Some(name) = name {
            reordered_columns.push(Some(name.clone()));

            if !inserted && name == from {
                insert = true;
            }
        } else {
            reordered_columns.push(None);
        }

        if insert {
            for column in columns {
                reordered_columns.push(Some(column.clone()));
            }
            inserted = true;
        }
    }

    Ok(select_fields(
        table,
        &reordered_columns
            .into_iter()
            .filter_map(|v| v)
            .collect::<Vec<_>>(),
        &table.tag,
    ))
}

fn move_before(table: &Value, columns: &[String], from: &ColumnPath) -> Result<Value, ShellError> {
    let from_fields = from.maybe_span().unwrap_or_else(Span::unknown);
    let from = if let Some((last, _)) = from.split_last() {
        last.as_string()
    } else {
        return Err(ShellError::labeled_error(
            "unknown column",
            "unknown column",
            from_fields,
        ));
    };

    let columns_moved = table.data_descriptors().into_iter().map(|name| {
        if columns.contains(&name) {
            None
        } else {
            Some(name)
        }
    });

    let mut reordered_columns = vec![];
    let mut inserted = false;

    for name in columns_moved.into_iter() {
        if let Some(name) = name {
            if !inserted && name == from {
                for column in columns {
                    reordered_columns.push(Some(column.clone()));
                }

                inserted = true;
            }

            reordered_columns.push(Some(name.clone()));
        } else {
            reordered_columns.push(None);
        }
    }

    Ok(select_fields(
        table,
        &reordered_columns
            .into_iter()
            .filter_map(|v| v)
            .collect::<Vec<_>>(),
        &table.tag,
    ))
}
