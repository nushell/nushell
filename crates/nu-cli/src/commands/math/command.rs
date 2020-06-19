use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "math"
    }

    fn signature(&self) -> Signature {
        Signature::build("math")
    }

    fn usage(&self) -> &str {
        "Use mathematical functions as aggregate functions on a list of numbers or tables"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(crate::commands::help::get_help(&Command, &registry.clone()))
                .into_value(Tag::unknown()),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::math::{
        average::average, max::maximum, min::minimum, sum::summation, utils::MathFunction,
    };
    use nu_plugin::test_helpers::value::{decimal, int};
    use nu_protocol::Value;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Command {})
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
                description: "Empty data should throw an error",
                values: Vec::new(),
                expected_err: Some(ShellError::unexpected("Expected data")),
                expected_res: Vec::new(),
            },
            TestCase {
                description: "Single value",
                values: vec![int(10)],
                expected_err: None,
                expected_res: vec![Ok(decimal(10)), Ok(int(10)), Ok(int(10)), Ok(int(10))],
            },
            TestCase {
                description: "Multiple Values",
                values: vec![int(10), int(30), int(20)],
                expected_err: None,
                expected_res: vec![Ok(decimal(20)), Ok(int(10)), Ok(int(30)), Ok(int(60))],
            },
            TestCase {
                description: "Mixed Values",
                values: vec![int(10), decimal(26.5), decimal(26.5)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(21)),
                    Ok(int(10)),
                    Ok(decimal(26.5)),
                    Ok(decimal(63)),
                ],
            },
            TestCase {
                description: "Negative Values",
                values: vec![int(10), int(-11), int(-14)],
                expected_err: None,
                expected_res: vec![Ok(decimal(-5)), Ok(int(-14)), Ok(int(10)), Ok(int(-15))],
            },
            TestCase {
                description: "Mixed Negative Values",
                values: vec![int(10), decimal(-11.5), decimal(-13.5)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(-5)),
                    Ok(decimal(-13.5)),
                    Ok(int(10)),
                    Ok(decimal(-15)),
                ],
            },
            // TODO-Uncomment once I figure out how to structure tables
            // TestCase {
            //     description: "Tables",
            //     values: vec![
            //         table(&vec![int(3), int(4), int(4)]),
            //         table(&vec![int(3), int(4), int(4)]),
            //         table(&vec![int(3), int(4), int(4)]),
            //     ],
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
            let math_functions: Vec<MathFunction> = vec![average, minimum, maximum, summation];
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
