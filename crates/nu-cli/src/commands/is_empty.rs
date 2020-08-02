use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

enum IsEmptyFor {
    Value,
    RowWithFieldsAndFallback(Vec<Tagged<ColumnPath>>, Value),
    RowWithField(Tagged<ColumnPath>),
    RowWithFieldAndFallback(Box<Tagged<ColumnPath>>, Value),
}

pub struct IsEmpty;

#[derive(Deserialize)]
pub struct IsEmptyArgs {
    rest: Vec<Value>,
}

#[async_trait]
impl WholeStreamCommand for IsEmpty {
    fn name(&self) -> &str {
        "empty?"
    }

    fn signature(&self) -> Signature {
        Signature::build("empty?").rest(
            SyntaxShape::Any,
            "the names of the columns to check emptiness followed by the replacement value.",
        )
    }

    fn usage(&self) -> &str {
        "Checks emptiness. The last value is the replacement value for any empty column(s) given to check against the table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        is_empty(args, registry).await
    }
}

async fn is_empty(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (IsEmptyArgs { rest }, input) = args.process(&registry).await?;

    Ok(input
        .map(move |value| {
            let value_tag = value.tag();

            let action = if rest.len() <= 2 {
                let field = rest.get(0);
                let replacement_if_true = rest.get(1);

                match (field, replacement_if_true) {
                    (Some(field), Some(replacement_if_true)) => {
                        IsEmptyFor::RowWithFieldAndFallback(
                            Box::new(field.as_column_path()?),
                            replacement_if_true.clone(),
                        )
                    }
                    (Some(field), None) => IsEmptyFor::RowWithField(field.as_column_path()?),
                    (_, _) => IsEmptyFor::Value,
                }
            } else {
                // let no_args = vec![];
                let mut arguments = rest.iter().rev();
                let replacement_if_true = match arguments.next() {
                    Some(arg) => arg.clone(),
                    None => UntaggedValue::boolean(value.is_empty()).into_value(&value_tag),
                };

                IsEmptyFor::RowWithFieldsAndFallback(
                    arguments
                        .map(|a| a.as_column_path())
                        .filter_map(Result::ok)
                        .collect(),
                    replacement_if_true,
                )
            };

            match action {
                IsEmptyFor::Value => Ok(ReturnSuccess::Value(
                    UntaggedValue::boolean(value.is_empty()).into_value(value_tag),
                )),
                IsEmptyFor::RowWithFieldsAndFallback(fields, default) => {
                    let mut out = value;

                    for field in fields.iter() {
                        let val = crate::commands::get::get_column_path(&field, &out)?;

                        let emptiness_value = match out {
                            obj
                            @
                            Value {
                                value: UntaggedValue::Row(_),
                                ..
                            } => {
                                if val.is_empty() {
                                    obj.replace_data_at_column_path(&field, default.clone())
                                        .ok_or_else(|| {
                                            ShellError::labeled_error(
                                                "empty? could not find place to check emptiness",
                                                "column name",
                                                &field.tag,
                                            )
                                        })
                                } else {
                                    Ok(obj)
                                }
                            }
                            _ => Err(ShellError::labeled_error(
                                "Unrecognized type in stream",
                                "original value",
                                &value_tag,
                            )),
                        };

                        out = emptiness_value?;
                    }

                    Ok(ReturnSuccess::Value(out))
                }
                IsEmptyFor::RowWithField(field) => {
                    let val = crate::commands::get::get_column_path(&field, &value)?;

                    match &value {
                        obj
                        @
                        Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => {
                            if val.is_empty() {
                                match obj.replace_data_at_column_path(
                                    &field,
                                    UntaggedValue::boolean(true).into_value(&value_tag),
                                ) {
                                    Some(v) => Ok(ReturnSuccess::Value(v)),
                                    None => Err(ShellError::labeled_error(
                                        "empty? could not find place to check emptiness",
                                        "column name",
                                        &field.tag,
                                    )),
                                }
                            } else {
                                Ok(ReturnSuccess::Value(value))
                            }
                        }
                        _ => Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            &value_tag,
                        )),
                    }
                }
                IsEmptyFor::RowWithFieldAndFallback(field, default) => {
                    let val = crate::commands::get::get_column_path(&field, &value)?;

                    match &value {
                        obj
                        @
                        Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => {
                            if val.is_empty() {
                                match obj.replace_data_at_column_path(&field, default) {
                                    Some(v) => Ok(ReturnSuccess::Value(v)),
                                    None => Err(ShellError::labeled_error(
                                        "empty? could not find place to check emptiness",
                                        "column name",
                                        &field.tag,
                                    )),
                                }
                            } else {
                                Ok(ReturnSuccess::Value(value))
                            }
                        }
                        _ => Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            &value_tag,
                        )),
                    }
                }
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::IsEmpty;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(IsEmpty {})
    }
}
