use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

enum IsEmptyFor {
    Value,
    RowWithFieldsAndFallback(Vec<Tagged<ColumnPath>>, Value),
    RowWithField(Tagged<ColumnPath>),
    RowWithFieldAndFallback(Box<Tagged<ColumnPath>>, Value),
}

pub struct IsEmpty;

impl PerItemCommand for IsEmpty {
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

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        value: Value,
    ) -> Result<OutputStream, ShellError> {
        let value_tag = value.tag();

        let action = if call_info.args.len() <= 2 {
            let field = call_info.args.expect_nth(0);
            let replacement_if_true = call_info.args.expect_nth(1);

            match (field, replacement_if_true) {
                (Ok(field), Ok(replacement_if_true)) => IsEmptyFor::RowWithFieldAndFallback(
                    Box::new(field.as_column_path()?),
                    replacement_if_true.clone(),
                ),
                (Ok(field), Err(_)) => IsEmptyFor::RowWithField(field.as_column_path()?),
                (_, _) => IsEmptyFor::Value,
            }
        } else {
            let no_args = vec![];
            let mut arguments = call_info
                .args
                .positional
                .as_ref()
                .unwrap_or_else(|| &no_args)
                .iter()
                .rev();
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
            IsEmptyFor::Value => Ok(futures::stream::iter(vec![Ok(ReturnSuccess::Value(
                UntaggedValue::boolean(value.is_empty()).into_value(value_tag),
            ))])
            .to_output_stream()),
            IsEmptyFor::RowWithFieldsAndFallback(fields, default) => {
                let mut out = value;

                for field in fields.iter() {
                    let val =
                        out.get_data_by_column_path(&field, Box::new(move |(_, _, err)| err))?;

                    let emptiness_value = match out {
                        obj
                        @
                        Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => {
                            if val.is_empty() {
                                match obj.replace_data_at_column_path(&field, default.clone()) {
                                    Some(v) => Ok(v),
                                    None => Err(ShellError::labeled_error(
                                        "empty? could not find place to check emptiness",
                                        "column name",
                                        &field.tag,
                                    )),
                                }
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

                Ok(futures::stream::iter(vec![Ok(ReturnSuccess::Value(out))]).to_output_stream())
            }
            IsEmptyFor::RowWithField(field) => {
                let val =
                    value.get_data_by_column_path(&field, Box::new(move |(_, _, err)| err))?;

                let stream = match &value {
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
                                Some(v) => futures::stream::iter(vec![Ok(ReturnSuccess::Value(v))]),
                                None => {
                                    return Err(ShellError::labeled_error(
                                        "empty? could not find place to check emptiness",
                                        "column name",
                                        &field.tag,
                                    ))
                                }
                            }
                        } else {
                            futures::stream::iter(vec![Ok(ReturnSuccess::Value(value))])
                        }
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            &value_tag,
                        ))
                    }
                };

                Ok(stream.to_output_stream())
            }
            IsEmptyFor::RowWithFieldAndFallback(field, default) => {
                let val =
                    value.get_data_by_column_path(&field, Box::new(move |(_, _, err)| err))?;

                let stream = match &value {
                    obj
                    @
                    Value {
                        value: UntaggedValue::Row(_),
                        ..
                    } => {
                        if val.is_empty() {
                            match obj.replace_data_at_column_path(&field, default) {
                                Some(v) => futures::stream::iter(vec![Ok(ReturnSuccess::Value(v))]),
                                None => {
                                    return Err(ShellError::labeled_error(
                                        "empty? could not find place to check emptiness",
                                        "column name",
                                        &field.tag,
                                    ))
                                }
                            }
                        } else {
                            futures::stream::iter(vec![Ok(ReturnSuccess::Value(value))])
                        }
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            &value_tag,
                        ))
                    }
                };

                Ok(stream.to_output_stream())
            }
        }
    }
}
