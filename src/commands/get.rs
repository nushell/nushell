use crate::commands::WholeStreamCommand;
use crate::data::base::shape::Shapes;
use crate::prelude::*;
use futures_util::pin_mut;
use indexmap::set::IndexSet;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{
    did_you_mean, ColumnPath, PathMember, ReturnSuccess, ReturnValue, Signature, SyntaxShape,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::span_for_spanned_list;
use nu_value_ext::get_data_by_column_path;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    rest: Vec<ColumnPath>,
}

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, get)?.run()
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

pub fn get(
    GetArgs { rest: mut fields }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if fields.is_empty() {
        let stream = async_stream! {
            let values = input.values;
            pin_mut!(values);

            let mut shapes = Shapes::new();
            let mut index = 0;

            while let Some(row) = values.next().await {
                shapes.add(&row, index);
                index += 1;
            }

            for row in shapes.to_values() {
                yield ReturnSuccess::value(row);
            }
        };

        let stream: BoxStream<'static, ReturnValue> = stream.boxed();

        Ok(stream.to_output_stream())
    } else {
        let member = fields.remove(0);
        trace!("get {:?} {:?}", member, fields);
        let stream = input
            .values
            .map(move |item| {
                let mut result = VecDeque::new();

                let member = vec![member.clone()];

                let column_paths = vec![&member, &fields]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<&ColumnPath>>();

                for path in column_paths {
                    let res = get_column_path(&path, &item);

                    match res {
                        Ok(got) => match got {
                            Value {
                                value: UntaggedValue::Table(rows),
                                ..
                            } => {
                                for item in rows {
                                    result.push_back(ReturnSuccess::value(item.clone()));
                                }
                            }
                            other => result.push_back(ReturnSuccess::value(other.clone())),
                        },
                        Err(reason) => result.push_back(ReturnSuccess::value(
                            UntaggedValue::Error(reason).into_untagged_value(),
                        )),
                    }
                }

                result
            })
            .flatten();

        Ok(stream.to_output_stream())
    }
}
