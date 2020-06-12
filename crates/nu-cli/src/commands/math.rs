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
            "The mathematical function that aggregates the vector the numbers (average, min, max)",
        )
    }

    fn usage(&self) -> &str {
        "Use mathematical functions to aggregate vectors of numbers
        math average
        math min
        math max
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
            Some(&"min") => math_func = minimum,
            Some(&"max") => math_func = maximum,
            Some(s) => {
                // TODO: Figure out how to do spans for error reporting
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

fn minimum(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let min_func = reducer_for(Reduce::Minimum);
    min_func(Value::nothing(), values.to_vec())
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
    use nu_plugin::test_helpers::value::{decimal, int};

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Math {})
    }

    #[test]
    fn test_math_functions() {
        struct TestCase {
            description: &'static str,
            values: Vec<Value>,
            expected_err: Option<ShellError>,
            // Order is: avg, min, max
            expected_res: Vec<Result<Value, ShellError>>,
        }
        let tt: Vec<TestCase> = vec![
            TestCase {
                description: "Single value",
                values: vec![int(10)],
                expected_err: None,
                expected_res: vec![Ok(decimal(10)), Ok(int(10)), Ok(int(10))],
            },
            TestCase {
                description: "Multiple Values",
                values: vec![int(10), int(30), int(20)],
                expected_err: None,
                expected_res: vec![Ok(decimal(20)), Ok(int(10)), Ok(int(30))],
            },
            TestCase {
                description: "Mixed Values",
                values: vec![int(10), decimal(26.5), decimal(26.5)],
                expected_err: None,
                expected_res: vec![Ok(decimal(21)), Ok(int(10)), Ok(decimal(26.5))],
            },
            TestCase {
                description: "Negative Values",
                values: vec![int(10), int(-11), int(-14)],
                expected_err: None,
                expected_res: vec![Ok(decimal(-5)), Ok(int(-14)), Ok(int(10))],
            },
            // TODO-Address once we figure out how to handle this. Maybe it's not an important use-case
            // TestCase {
            //     description: "Mixed Negative Values",
            //     values: vec![int(10), decimal(-11.5), decimal(-13.5)],
            //     expected_err: None,
            //     expected_res: vec![Ok(decimal(-5)), Ok(decimal(-13.5)), Ok(int(10))],
            // },
            // TODO-Uncomment once Issue: https://github.com/nushell/nushell/issues/1883 is resolved
            // TestCase {
            //     description: "Invalid Mixed Values",
            //     values: vec![int(10), decimal(26.5), decimal(26.5), string("math")],
            //     expected_err: Some(ShellError::unimplemented("something")),
            //     expected_res: vec![],
            // },
        ];
        let test_tag = Tag::unknown();

        for tc in tt.iter() {
            let tc: &TestCase = tc; // Just for type annotations
            let math_functions: Vec<MathFunction> = vec![avg, minimum, maximum];
            let results = math_functions
                .iter()
                .map(|mf| mf(&tc.values, &test_tag))
                .collect_vec();

            if tc.expected_err.is_some() {
                assert!(
                    results.iter().all(|r| r.is_err()),
                    "Expected all functions to error for test-case: {}",
                    tc.description,
                );
            } else {
                for (i, res) in results.into_iter().enumerate() {
                    assert_eq!(
                        res, tc.expected_res[i],
                        "math function {} failed on test-case {}",
                        i, tc.description
                    );
                }
            }
        }
    }
}
