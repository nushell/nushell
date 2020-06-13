use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::set::IndexSet;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{
    did_you_mean, ColumnPath, PathMember, Primitive, ReturnSuccess, Signature, SyntaxShape,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::span_for_spanned_list;
use nu_value_ext::get_data_by_column_path;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn signature(&self) -> Signature {
        Signature::build("get").rest(
            SyntaxShape::ColumnPath,
            "optionally return additional data by path",
        )
    }

    fn usage(&self) -> &str {
        "Open given cells as text."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        get(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Extract the name of files as a list",
                example: "ls | get name",
                result: None,
            },
            Example {
                description: "Extract the cpu list from the sys information",
                example: "sys | get cpu",
                result: None,
            },
        ]
    }
}

pub fn get_column_path(path: &ColumnPath, obj: &Value) -> Result<Value, ShellError> {
    let fields = path.clone();

    get_data_by_column_path(
        obj,
        path,
        Box::new(move |(obj_source, column_path_tried, error)| {
            let path_members_span = span_for_spanned_list(fields.members().iter().map(|p| p.span));

            match &obj_source.value {
                UntaggedValue::Table(rows) => match column_path_tried {
                    PathMember {
                        unspanned: UnspannedPathMember::String(column),
                        ..
                    } => {
                        let primary_label = format!("There isn't a column named '{}'", &column);

                        let suggestions: IndexSet<_> = rows
                            .iter()
                            .filter_map(|r| did_you_mean(&r, &column_path_tried))
                            .map(|s| s[0].1.to_owned())
                            .collect();
                        let mut existing_columns: IndexSet<_> = IndexSet::default();
                        let mut names: Vec<String> = vec![];

                        for row in rows {
                            for field in row.data_descriptors() {
                                if !existing_columns.contains(&field[..]) {
                                    existing_columns.insert(field.clone());
                                    names.push(field);
                                }
                            }
                        }

                        if names.is_empty() {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                "Appears to contain rows. Try indexing instead.",
                                column_path_tried.span.since(path_members_span),
                            );
                        } else {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                format!(
                                    "Perhaps you meant '{}'? Columns available: {}",
                                    suggestions
                                        .iter()
                                        .map(|x| x.to_owned())
                                        .collect::<Vec<String>>()
                                        .join(","),
                                    names.join(",")
                                ),
                                column_path_tried.span.since(path_members_span),
                            );
                        };
                    }
                    PathMember {
                        unspanned: UnspannedPathMember::Int(idx),
                        ..
                    } => {
                        let total = rows.len();

                        let secondary_label = if total == 1 {
                            "The table only has 1 row".to_owned()
                        } else {
                            format!("The table only has {} rows (0 to {})", total, total - 1)
                        };

                        return ShellError::labeled_error_with_secondary(
                            "Row not found",
                            format!("There isn't a row indexed at {}", idx),
                            column_path_tried.span,
                            secondary_label,
                            column_path_tried.span.since(path_members_span),
                        );
                    }
                },
                UntaggedValue::Row(columns) => match column_path_tried {
                    PathMember {
                        unspanned: UnspannedPathMember::String(column),
                        ..
                    } => {
                        let primary_label = format!("There isn't a column named '{}'", &column);

                        if let Some(suggestions) = did_you_mean(&obj_source, column_path_tried) {
                            return ShellError::labeled_error_with_secondary(
                                "Unknown column",
                                primary_label,
                                column_path_tried.span,
                                format!(
                                    "Perhaps you meant '{}'? Columns available: {}",
                                    suggestions[0].1,
                                    &obj_source.data_descriptors().join(",")
                                ),
                                column_path_tried.span.since(path_members_span),
                            );
                        }
                    }
                    PathMember {
                        unspanned: UnspannedPathMember::Int(idx),
                        ..
                    } => {
                        return ShellError::labeled_error_with_secondary(
                            "No rows available",
                            format!("A row at '{}' can't be indexed.", &idx),
                            column_path_tried.span,
                            format!(
                                "Appears to contain columns. Columns available: {}",
                                columns.keys().join(",")
                            ),
                            column_path_tried.span.since(path_members_span),
                        )
                    }
                },
                _ => {}
            }

            if let Some(suggestions) = did_you_mean(&obj_source, column_path_tried) {
                return ShellError::labeled_error(
                    "Unknown column",
                    format!("did you mean '{}'?", suggestions[0].1),
                    column_path_tried.span.since(path_members_span),
                );
            }

            error
        }),
    )
}

pub async fn get(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (GetArgs { rest: mut fields }, mut input) = args.process(&registry).await?;
    if fields.is_empty() {
        let vec = input.drain_vec().await;

        let descs = nu_protocol::merge_descriptors(&vec);

        Ok(futures::stream::iter(descs.into_iter().map(ReturnSuccess::value)).to_output_stream())
    } else {
        let member = fields.remove(0);
        trace!("get {:?} {:?}", member, fields);

        Ok(input
            .map(move |item| {
                let member = vec![member.clone()];

                let column_paths = vec![&member, &fields]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<&ColumnPath>>();

                let mut output = vec![];
                for path in column_paths {
                    let res = get_column_path(&path, &item);

                    match res {
                        Ok(got) => match got {
                            Value {
                                value: UntaggedValue::Table(rows),
                                ..
                            } => {
                                for item in rows {
                                    output.push(ReturnSuccess::value(item.clone()));
                                }
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Nothing),
                                ..
                            } => {}
                            other => output.push(ReturnSuccess::value(other.clone())),
                        },
                        Err(reason) => output.push(ReturnSuccess::value(
                            UntaggedValue::Error(reason).into_untagged_value(),
                        )),
                    }
                }

                futures::stream::iter(output)
            })
            .flatten()
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Get;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Get {})
    }
}
