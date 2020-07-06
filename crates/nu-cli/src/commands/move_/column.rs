use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::base::select_fields;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::span_for_spanned_list;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    rest: Vec<ColumnPath>,
    after: Option<ColumnPath>,
    before: Option<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "move column"
    }

    fn signature(&self) -> Signature {
        Signature::build("move column")
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }
}

async fn operate(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name = raw_args.call_info.name_tag.clone();
    let registry = registry.clone();
    let (
        Arguments {
            rest: mut columns,
            before,
            after,
        },
        input,
    ) = raw_args.process(&registry).await?;

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

                let after_span = span_for_spanned_list(after.members().iter().map(|p| p.span));

                if after.members().len() == 1 {
                    let keys = column_paths
                        .iter()
                        .filter_map(|c| c.last())
                        .map(|c| c.as_string())
                        .collect::<Vec<_>>();

                    if let Some(column) = after.last() {
                        if !keys.contains(&column.as_string()) {
                            ReturnSuccess::value(move_after(&item, &keys, &after, &name)?)
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

                let before_span = span_for_spanned_list(before.members().iter().map(|p| p.span));

                if before.members().len() == 1 {
                    let keys = column_paths
                        .iter()
                        .filter_map(|c| c.last())
                        .map(|c| c.as_string())
                        .collect::<Vec<_>>();

                    if let Some(column) = before.last() {
                        if !keys.contains(&column.as_string()) {
                            ReturnSuccess::value(move_before(&item, &keys, &before, &name)?)
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

fn move_after(
    table: &Value,
    columns: &[String],
    from: &ColumnPath,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let from_fields = span_for_spanned_list(from.members().iter().map(|p| p.span));
    let from = if let Some((last, _)) = from.split_last() {
        last.as_string()
    } else {
        return Err(ShellError::labeled_error(
            "unknown column",
            "unknown column",
            from_fields,
        ));
    };

    let columns_moved = table
        .data_descriptors()
        .into_iter()
        .map(|name| {
            if columns.contains(&name) {
                None
            } else {
                Some(name)
            }
        })
        .collect::<Vec<_>>();

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
        &tag,
    ))
}

fn move_before(
    table: &Value,
    columns: &[String],
    from: &ColumnPath,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let from_fields = span_for_spanned_list(from.members().iter().map(|p| p.span));
    let from = if let Some((last, _)) = from.split_last() {
        last.as_string()
    } else {
        return Err(ShellError::labeled_error(
            "unknown column",
            "unknown column",
            from_fields,
        ));
    };

    let columns_moved = table
        .data_descriptors()
        .into_iter()
        .map(|name| {
            if columns.contains(&name) {
                None
            } else {
                Some(name)
            }
        })
        .collect::<Vec<_>>();

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
        &tag,
    ))
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
