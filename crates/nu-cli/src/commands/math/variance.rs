use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::data::value::compute_values;
use crate::prelude::*;
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::{hir::Operator, Primitive, Signature, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math variance"
    }

    fn signature(&self) -> Signature {
        Signature::build("math variance")
    }

    fn usage(&self) -> &str {
        "Finds the variance of a list of numbers or tables"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_with_function(
            RunnableContext {
                input: args.input,
                registry: registry.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
                raw_input: args.raw_input,
            },
            variance,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the variance of a list of numbers",
            example: "echo [1 2 3 4 5] | math variance",
            result: Some(vec![UntaggedValue::decimal(2).into()]),
        }]
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
                UntaggedValue::from(Primitive::Int(num.clone().into()))
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

pub fn variance(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let ss = sum_of_squares(values, name)?;
    let n = BigDecimal::from_usize(values.len()).ok_or_else(|| {
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
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
