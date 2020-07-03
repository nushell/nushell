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
        avg::average, max::maximum, median::median, min::minimum, mode::mode, sum::summation,
        utils::calculate, utils::MathFunction,
    };
    use nu_plugin::row;
    use nu_plugin::test_helpers::value::{decimal, int, table};
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
            // Order is: average, minimum, maximum, median, summation
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
                expected_res: vec![
                    Ok(decimal(10)),
                    Ok(int(10)),
                    Ok(int(10)),
                    Ok(int(10)),
                    Ok(table(&[int(10)])),
                    Ok(int(10)),
                ],
            },
            TestCase {
                description: "Multiple Values",
                values: vec![int(10), int(20), int(30)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(20)),
                    Ok(int(10)),
                    Ok(int(30)),
                    Ok(int(20)),
                    Ok(table(&[int(10), int(20), int(30)])),
                    Ok(int(60)),
                ],
            },
            TestCase {
                description: "Mixed Values",
                values: vec![int(10), decimal(26.5), decimal(26.5)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(21)),
                    Ok(int(10)),
                    Ok(decimal(26.5)),
                    Ok(decimal(26.5)),
                    Ok(table(&[decimal(26.5)])),
                    Ok(decimal(63)),
                ],
            },
            TestCase {
                description: "Negative Values",
                values: vec![int(-14), int(-11), int(10)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(-5)),
                    Ok(int(-14)),
                    Ok(int(10)),
                    Ok(int(-11)),
                    Ok(table(&[int(-14), int(-11), int(10)])),
                    Ok(int(-15)),
                ],
            },
            TestCase {
                description: "Mixed Negative Values",
                values: vec![decimal(-13.5), decimal(-11.5), int(10)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal(-5)),
                    Ok(decimal(-13.5)),
                    Ok(int(10)),
                    Ok(decimal(-11.5)),
                    Ok(table(&[decimal(-13.5), decimal(-11.5), int(10)])),
                    Ok(decimal(-15)),
                ],
            },
            TestCase {
                description: "Tables Or Rows",
                values: vec![
                    row!["col1".to_owned() => int(1), "col2".to_owned() => int(5)],
                    row!["col1".to_owned() => int(2), "col2".to_owned() => int(6)],
                    row!["col1".to_owned() => int(3), "col2".to_owned() => int(7)],
                    row!["col1".to_owned() => int(4), "col2".to_owned() => int(8)],
                ],
                expected_err: None,
                expected_res: vec![
                    Ok(row!["col1".to_owned() => decimal(2.5), "col2".to_owned() => decimal(6.5)]),
                    Ok(row!["col1".to_owned() => int(1), "col2".to_owned() => int(5)]),
                    Ok(row!["col1".to_owned() => int(4), "col2".to_owned() => int(8)]),
                    Ok(row!["col1".to_owned() => decimal(2.5), "col2".to_owned() => decimal(6.5)]),
                    Ok(row![
                        "col1".to_owned() => table(&[int(1), int(2), int(3), int(4)]),
                        "col2".to_owned() => table(&[int(5), int(6), int(7), int(8)])
                    ]),
                    Ok(row!["col1".to_owned() => int(10), "col2".to_owned() => int(26)]),
                ],
            },
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
            let math_functions: Vec<MathFunction> =
                vec![average, minimum, maximum, median, mode, summation];
            let results = math_functions
                .into_iter()
                .map(|mf| calculate(&tc.values, &test_tag, mf))
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
