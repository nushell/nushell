use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::hir::{convert_number_to_u64, Number, Operator};
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use num_traits::identities::Zero;

use indexmap::map::IndexMap;

pub struct Math;

type MathFunction = fn(values: &[Value], tag: &Tag) -> Result<Value, ShellError>;

#[async_trait]
impl WholeStreamCommand for Math {
    fn name(&self) -> &str {
        "math"
    }

    fn signature(&self) -> Signature {
        Signature::build("math").required(
            "operation",
            SyntaxShape::String,
            "The mathematical function that aggregates the vector the numbers (average, max, min)",
        )
    }

    fn usage(&self) -> &str {
        "Use mathematical functions to aggregate vectors of numbers
        math average
        math max
        math min
        "
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        // TODO: Kind of a hack? But don't currently know a better way to get sub-commands of a command
        let sub_args = args.raw_input.split_whitespace().collect_vec();
        let math_func: MathFunction;
        match sub_args.last() {
            Some(&"average") => math_func = avg,
            Some(&"max") => math_func = maximum,
            Some(&"minimum") => unimplemented!(),
            Some(s) => {
                return Err(ShellError::unexpected(format!(
                    "Unexpected math function: {}",
                    s
                )));
            }
            None => math_func = avg,
        }
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
            math_func,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Math a list of numbers",
                example: "echo [-50 100.0 25] | math average",
                result: Some(vec![UntaggedValue::decimal(25).into()]),
            },
            // Example {
            //     description: "Find the maximum of a list of numbers",
            //     example: "echo [-50 100 25] | math max",
            //     result: Some(vec![UntaggedValue::decimal(100).into()]),
            // },
            // Example {
            //     description: "Find the minimum of a list of numbers",
            //     example: "echo [-50 100 0] | math min",
            //     result: Some(vec![UntaggedValue::decimal(-50).into()]),
            // },
            // TODO: Add a example showing how it would work with a tables
        ]
    }
}

async fn calculate(
    RunnableContext {
        mut input, name, ..
    }: RunnableContext,
    mf: MathFunction,
) -> Result<OutputStream, ShellError> {
    let values: Vec<Value> = input.drain_vec().await;

    if values.iter().all(|v| v.is_primitive()) {
        match mf(&values, &name) {
            Ok(result) => Ok(OutputStream::one(ReturnSuccess::value(result))),
            Err(err) => Err(err),
        }
    } else {
        let mut column_values = IndexMap::new();
        for value in values {
            if let UntaggedValue::Row(row_dict) = value.value {
                for (key, value) in row_dict.entries.iter() {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert(vec![value.clone()]);
                }
            }
        }

        let mut column_totals = IndexMap::new();
        for (col_name, col_vals) in column_values {
            match mf(&col_vals, &name) {
                Ok(result) => {
                    column_totals.insert(col_name, result);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(Dictionary {
                entries: column_totals,
            })
            .into_untagged_value(),
        )))
    }
}

fn maximum(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(Value::nothing(), values.to_vec())
}

fn avg(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Sum);

    let number = BigDecimal::from_usize(values.len()).expect("expected a usize-sized bigdecimal");

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
    use super::*;
    use nu_plugin::test_helpers::value::{decimal, int, string, table};

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Math {})
    }

    #[test]
    fn test_max() {
        maximum(&Vec::new(), &Tag::unknown()).expect_err("Empty data should produce an error");

        let test_tag = Tag::unknown();
        assert_eq!(maximum(&vec![int(10)], &test_tag), Ok(int(10)),);
        assert_eq!(
            maximum(&vec![int(10), int(30), int(20)], &test_tag),
            Ok(int(30),),
        );
        assert_eq!(
            maximum(&vec![int(10), decimal(30), int(20)], &test_tag),
            Ok(decimal(30),),
        );

        // TODO: Get tables to work?
        // assert_eq!(
        //     maximum(
        //         &vec![table(&vec![int(30)]), table(&vec![int(50)]),],
        //         &test_tag
        //     ),
        //     Ok(int(50),),
        // );

        maximum(&vec![string("math")], &test_tag)
            .expect_err("String vector should return an error");
    }
}
