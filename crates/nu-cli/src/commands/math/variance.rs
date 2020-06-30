use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::data::value::compute_values;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::{FromPrimitive, Zero};
use nu_errors::ShellError;
use nu_protocol::{hir::Operator, Signature, UntaggedValue, Value};

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

fn sum_squared_values(values: &[Value]) -> Result<Value, ShellError> {
    let mut acc = Value::zero();
    for value in values {
        match value.value {
            UntaggedValue::Primitive(_) => {
                let v_squared = compute_values(Operator::Multiply, &value.value, &value.value);
                match v_squared {
                    Ok(v) => acc = acc + v.into_untagged_value(),
                    Err((left_type, right_type)) => {
                        return Err(ShellError::coerce_error(
                            left_type.spanned(value.tag.span),
                            right_type.spanned(value.tag.span),
                        ))
                    }
                }
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "Attempted to compute the sum of a value that cannot be summed.",
                    "value appears here",
                    value.tag.span,
                ))
            }
        }
    }
    Ok(acc)
}

pub fn variance(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    let number = BigDecimal::from_usize(values.len()).ok_or_else(|| {
        ShellError::labeled_error(
            "could not convert to big decimal",
            "could not convert to big decimal",
            &name.span,
        )
    })?;

    let total_rows = UntaggedValue::decimal(number);
    let total_sum = sum(Value::zero(), values.to_vec())?;
    let total_sum_squared_values = sum_squared_values(values)?;

    if total_sum.is_primitive() && total_sum_squared_values.is_primitive() {
        let sum_x = total_sum.value;
        let sum_x2 = total_sum_squared_values.value;
        // (SUM(X))^2
        let sum_x_2 = compute_values(Operator::Multiply, &sum_x, &sum_x).unwrap();
        // (SUM(X))^2 / N
        let sum_x_2_div_n = compute_values(Operator::Divide, &sum_x_2, &total_rows).unwrap();
        // SS = SUM(X^2) - (SUM(X))^2/N
        let ss = compute_values(Operator::Minus, &sum_x2, &sum_x_2_div_n).unwrap();
        // Variance = SS / N
        let result = compute_values(Operator::Divide, &ss, &total_rows);

        match result {
            Ok(value) => Ok(value.into_value(name)),
            Err((left_type, right_type)) => Err(ShellError::coerce_error(
                left_type.spanned(name.span),
                right_type.spanned(name.span),
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "could not calculate variance of non-integer or unrelated types",
            "source",
            name,
        ))
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
