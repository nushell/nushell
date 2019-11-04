use crate::commands::WholeStreamCommand;
use crate::data::base::shape::Shapes;
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;
use crate::utils::did_you_mean;
use crate::ColumnPath;
use futures_util::pin_mut;
use log::trace;

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

pub fn get_column_path(
    path: &ColumnPath,
    obj: &Tagged<Value>,
) -> Result<Tagged<Value>, ShellError> {
    let fields = path.clone();

    let value = obj.get_data_by_column_path(
        path,
        Box::new(move |(obj_source, column_path_tried, error)| {
            match obj_source {
                Value::Table(rows) => {
                    let total = rows.len();
                    let end_tag = match fields
                        .members()
                        .iter()
                        .nth_back(if fields.members().len() > 2 { 1 } else { 0 })
                    {
                        Some(last_field) => last_field.span(),
                        None => column_path_tried.span(),
                    };

                    return ShellError::labeled_error_with_secondary(
                        "Row not found",
                        format!("There isn't a row indexed at {}", **column_path_tried),
                        column_path_tried.span(),
                        if total == 1 {
                            format!("The table only has 1 row")
                        } else {
                            format!("The table only has {} rows (0 to {})", total, total - 1)
                        },
                        end_tag,
                    );
                }
                _ => {}
            }

            match did_you_mean(&obj_source, column_path_tried) {
                Some(suggestions) => {
                    return ShellError::labeled_error(
                        "Unknown column",
                        format!("did you mean '{}'?", suggestions[0].1),
                        span_for_spanned_list(fields.members().iter().map(|p| p.span())),
                    )
                }
                None => {}
            }

            return error;
        }),
    );

    let res = match value {
        Ok(Tagged { item: v, tag }) => Ok((v.clone()).tagged(&tag)),
        Err(reason) => Err(reason),
    };

    res
}

pub fn get(
    GetArgs { rest: mut fields }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if fields.len() == 0 {
        let stream = async_stream! {
            let values = input.values;
            pin_mut!(values);

            let mut shapes = Shapes::new();
            let mut index = 0;

            while let Some(row) = values.next().await {
                shapes.add(&row.item, index);
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
                            Tagged {
                                item: Value::Table(rows),
                                ..
                            } => {
                                for item in rows {
                                    result.push_back(ReturnSuccess::value(item.clone()));
                                }
                            }
                            other => result.push_back(ReturnSuccess::value(other.clone())),
                        },
                        Err(reason) => result
                            .push_back(ReturnSuccess::value(Value::Error(reason).tagged_unknown())),
                    }
                }

                result
            })
            .flatten();

        Ok(stream.to_output_stream())
    }
}
