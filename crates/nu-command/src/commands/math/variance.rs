use crate::prelude::*;
use bigdecimal::FromPrimitive;
use nu_data::value::compute_values;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Operator, Dictionary, Primitive, ReturnSuccess, Signature, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
struct Arguments {
    sample: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math variance"
    }

    fn signature(&self) -> Signature {
        Signature::build("math variance").switch("sample", "calculate sample variance", Some('s'))
    }

    fn usage(&self) -> &str {
        "Finds the variance of a list of numbers or tables"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let (Arguments { sample }, mut input) = args.process().await?;

        let values: Vec<Value> = input.drain_vec().await;

        let n = if let Tagged { item: true, .. } = sample {
            values.len() - 1
        } else {
            values.len()
        };

        let res = if values.iter().all(|v| v.is_primitive()) {
            compute_variance(&values, n, &name)
        } else {
            // If we are not dealing with Primitives, then perhaps we are dealing with a table
            // Create a key for each column name
            let mut column_values = IndexMap::new();
            for value in values {
                if let UntaggedValue::Row(row_dict) = &value.value {
                    for (key, value) in row_dict.entries.iter() {
                        column_values
                            .entry(key.clone())
                            .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                            .or_insert(vec![value.clone()]);
                    }
                }
            }
            // The mathematical function operates over the columns of the table
            let mut column_totals = IndexMap::new();
            for (col_name, col_vals) in column_values {
                if let Ok(out) = compute_variance(&col_vals, n, &name) {
                    column_totals.insert(col_name, out);
                }
            }

            if column_totals.keys().len() == 0 {
                return Err(ShellError::labeled_error(
                    "Attempted to compute values that can't be operated on",
                    "value appears here",
                    name.span,
                ));
            }

            Ok(UntaggedValue::Row(Dictionary {
                entries: column_totals,
            })
            .into_untagged_value())
        }?;

        if res.value.is_table() {
            Ok(OutputStream::from(
                res.table_entries()
                    .map(|v| ReturnSuccess::value(v.clone()))
                    .collect::<Vec<_>>(),
            ))
        } else {
            Ok(OutputStream::one(ReturnSuccess::value(res)))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the variance of a list of numbers",
                example: "echo [1 2 3 4 5] | math variance",
                result: Some(vec![UntaggedValue::decimal_from_float(
                    2.0,
                    Span::unknown(),
                )
                .into()]),
            },
            Example {
                description: "Get the sample variance of a list of numbers",
                example: "echo [1 2 3 4 5] | math variance -s",
                result: Some(vec![UntaggedValue::decimal_from_float(
                    2.5,
                    Span::unknown(),
                )
                .into()]),
            },
        ]
    }
}

fn sum_of_squares(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let n = BigDecimal::from_usize(values.len()).ok_or_else(|| {
        ShellError::labeled_error(
            "could not convert to big decimal",
            "could not convert to big decimal",
            &name.span,
        )
    })?;
    let mut sum_x = UntaggedValue::int(0).into_untagged_value();
    let mut sum_x2 = UntaggedValue::int(0).into_untagged_value();
    for value in values {
        let v = match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Filesize(num)),
                ..
            } => {
                UntaggedValue::from(Primitive::Int(num.clone()))
            },
            Value {
                value: UntaggedValue::Primitive(num),
                ..
            } => {
                UntaggedValue::from(num.clone())
            },
            _ => {
                return Err(ShellError::labeled_error(
                    "Attempted to compute the sum of squared values of a value that cannot be summed or squared.",
                    "value appears here",
                    value.tag.span,
                ))
            }
        };
        let v_squared = compute_values(Operator::Multiply, &v, &v);
        match v_squared {
            // X^2
            Ok(x2) => {
                sum_x2 = match compute_values(Operator::Plus, &sum_x2, &x2) {
                    Ok(v) => v.into_untagged_value(),
                    Err((left_type, right_type)) => {
                        return Err(ShellError::coerce_error(
                            left_type.spanned(name.span),
                            right_type.spanned(name.span),
                        ))
                    }
                };
            }
            Err((left_type, right_type)) => {
                return Err(ShellError::coerce_error(
                    left_type.spanned(value.tag.span),
                    right_type.spanned(value.tag.span),
                ))
            }
        };
        sum_x = match compute_values(Operator::Plus, &sum_x, &v) {
            Ok(v) => v.into_untagged_value(),
            Err((left_type, right_type)) => {
                return Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                ))
            }
        };
    }

    let sum_x_squared = match compute_values(Operator::Multiply, &sum_x, &sum_x) {
        Ok(v) => v.into_untagged_value(),
        Err((left_type, right_type)) => {
            return Err(ShellError::coerce_error(
                left_type.spanned(name.span),
                right_type.spanned(name.span),
            ))
        }
    };
    let sum_x_squared_div_n = match compute_values(Operator::Divide, &sum_x_squared, &n.into()) {
        Ok(v) => v.into_untagged_value(),
        Err((left_type, right_type)) => {
            return Err(ShellError::coerce_error(
                left_type.spanned(name.span),
                right_type.spanned(name.span),
            ))
        }
    };
    let ss = match compute_values(Operator::Minus, &sum_x2, &sum_x_squared_div_n) {
        Ok(v) => v.into_untagged_value(),
        Err((left_type, right_type)) => {
            return Err(ShellError::coerce_error(
                left_type.spanned(name.span),
                right_type.spanned(name.span),
            ))
        }
    };

    Ok(ss)
}

#[cfg(test)]
pub fn variance(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    compute_variance(values, values.len(), name)
}

pub fn compute_variance(values: &[Value], n: usize, name: &Tag) -> Result<Value, ShellError> {
    let ss = sum_of_squares(values, name)?;
    let n = BigDecimal::from_usize(n).ok_or_else(|| {
        ShellError::labeled_error(
            "could not convert to big decimal",
            "could not convert to big decimal",
            &name.span,
        )
    })?;
    let variance = compute_values(Operator::Divide, &ss, &n.into());
    match variance {
        Ok(value) => Ok(value.into_value(name)),
        Err((_, _)) => Err(ShellError::labeled_error(
            "could not calculate variance of non-integer or unrelated types",
            "source",
            name,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
