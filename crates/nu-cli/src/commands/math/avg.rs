use crate::commands::math::utils::calculate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::{FromPrimitive, Zero};
use nu_errors::ShellError;
use nu_protocol::{
    hir::{convert_number_to_u64, Number, Operator},
    Primitive, Signature, UntaggedValue, Value,
};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math avg"
    }

    fn signature(&self) -> Signature {
        Signature::build("math avg")
    }

    fn usage(&self) -> &str {
        "Finds the average of a list of numbers or tables"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        calculate(
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
            average,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the average of a list of numbers",
            example: "echo [-50 100.0 25] | math avg",
            result: Some(vec![UntaggedValue::decimal(25).into()]),
        }]
    }
}

pub fn average(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    let number = BigDecimal::from_usize(values.len()).ok_or_else(|| {
        ShellError::labeled_error(
            "could not convert to big decimal",
            "could not convert to big decimal",
            &name.span,
        )
    })?;

    let total_rows = UntaggedValue::decimal(number);
    let total = sum(Value::zero(), values.to_vec())?;

    match total {
        Value {
            value: UntaggedValue::Primitive(Primitive::Bytes(num)),
            ..
        } => {
            let left = UntaggedValue::from(Primitive::Int(num.into()));
            let result = crate::data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result))) => {
                    let number = Number::Decimal(result);
                    let number = convert_number_to_u64(&number);
                    Ok(UntaggedValue::bytes(number).into_value(name))
                }
                Ok(_) => Err(ShellError::labeled_error(
                    "could not calculate average of non-integer or unrelated types",
                    "source",
                    name,
                )),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                )),
            }
        }
        Value {
            value: UntaggedValue::Primitive(other),
            ..
        } => {
            let left = UntaggedValue::from(other);
            let result = crate::data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(value) => Ok(value.into_value(name)),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                )),
            }
        }
        _ => Err(ShellError::labeled_error(
            "could not calculate average of non-integer or unrelated types",
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
