use crate::prelude::*;
use nu_engine::WholeStreamCommand;
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(get_help(&Command, &args.scope)).into_value(Tag::unknown()),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::math::{
        avg::average, max::maximum, median::median, min::minimum, mode::mode, stddev::stddev,
        sum::summation, utils::calculate, utils::MathFunction, variance::variance,
    };
    use nu_protocol::{row, Value};
    use nu_test_support::value::{decimal, decimal_from_float, int, table};
    use std::str::FromStr;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
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
                    Ok(decimal_from_float(10.0)),
                    Ok(int(10)),
                    Ok(int(10)),
                    Ok(int(10)),
                    Ok(table(&[int(10)])),
                    Ok(decimal_from_float(0.0)),
                    Ok(int(10)),
                    Ok(decimal_from_float(0.0)),
                ],
            },
            TestCase {
                description: "Multiple Values",
                values: vec![int(10), int(20), int(30)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal_from_float(20.0)),
                    Ok(int(10)),
                    Ok(int(30)),
                    Ok(int(20)),
                    Ok(table(&[int(10), int(20), int(30)])),
                    Ok(decimal(BigDecimal::from_str("8.164965809277260327324280249019637973219824935522233761442308557503201258191050088466198110348800783").expect("Could not convert to decimal from string"))),
                    Ok(int(60)),
                    Ok(decimal(BigDecimal::from_str("66.66666666666666666666666666666666666666666666666666666666666666666666666666666666666666666666666667").expect("Could not convert to decimal from string"))),
                ],
            },
            TestCase {
                description: "Mixed Values",
                values: vec![int(10), decimal_from_float(26.5), decimal_from_float(26.5)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal_from_float(21.0)),
                    Ok(int(10)),
                    Ok(decimal_from_float(26.5)),
                    Ok(decimal_from_float(26.5)),
                    Ok(table(&[decimal_from_float(26.5)])),
                    Ok(decimal(BigDecimal::from_str("7.77817459305202276840928798315333943213319531457321440247173855894902863154158871367713143880202865").expect("Could not convert to decimal from string"))),
                    Ok(decimal_from_float(63.0)),
                    Ok(decimal_from_float(60.5)),
                ],
            },
            TestCase {
                description: "Negative Values",
                values: vec![int(-14), int(-11), int(10)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal_from_float(-5.0)),
                    Ok(int(-14)),
                    Ok(int(10)),
                    Ok(int(-11)),
                    Ok(table(&[int(-14), int(-11), int(10)])),
                    Ok(decimal(BigDecimal::from_str("10.67707825203131121081152396559571062628228776946058011397810604284900898365140801704064843595778374").expect("Could not convert to decimal from string"))),
                    Ok(int(-15)),
                    Ok(decimal_from_float(114.0)),
                ],
            },
            TestCase {
                description: "Mixed Negative Values",
                values: vec![decimal_from_float(-13.5), decimal_from_float(-11.5), int(10)],
                expected_err: None,
                expected_res: vec![
                    Ok(decimal_from_float(-5.0)),
                    Ok(decimal_from_float(-13.5)),
                    Ok(int(10)),
                    Ok(decimal_from_float(-11.5)),
                    Ok(table(&[decimal_from_float(-13.5), decimal_from_float(-11.5), int(10)])),
                    Ok(decimal(BigDecimal::from_str("10.63798226482196513098036125801342585449179971588207816421068645273754903468375890632981926875247027").expect("Could not convert to decimal from string"))),
                    Ok(decimal_from_float(-15.0)),
                    Ok(decimal(BigDecimal::from_str("113.1666666666666666666666666666666666666666666666666666666666666666666666666666666666666666666666667").expect("Could not convert to decimal from string"))),
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
                    Ok(row!["col1".to_owned() => decimal_from_float(2.5), "col2".to_owned() => decimal_from_float(6.5)]),
                    Ok(row!["col1".to_owned() => int(1), "col2".to_owned() => int(5)]),
                    Ok(row!["col1".to_owned() => int(4), "col2".to_owned() => int(8)]),
                    Ok(row!["col1".to_owned() => decimal_from_float(2.5), "col2".to_owned() => decimal_from_float(6.5)]),
                    Ok(row![
                        "col1".to_owned() => table(&[int(1), int(2), int(3), int(4)]),
                        "col2".to_owned() => table(&[int(5), int(6), int(7), int(8)])
                        ]),
                    Ok(row![
                        "col1".to_owned() => decimal(BigDecimal::from_str("1.118033988749894848204586834365638117720309179805762862135448622705260462818902449707207204189391137").expect("Could not convert to decimal from string")), 
                        "col2".to_owned() => decimal(BigDecimal::from_str("1.118033988749894848204586834365638117720309179805762862135448622705260462818902449707207204189391137").expect("Could not convert to decimal from string"))
                    ]),
                    Ok(row!["col1".to_owned() => int(10), "col2".to_owned() => int(26)]),
                    Ok(row!["col1".to_owned() => decimal_from_float(1.25), "col2".to_owned() => decimal_from_float(1.25)]),
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
            let math_functions: Vec<MathFunction> = vec![
                average, minimum, maximum, median, mode, stddev, summation, variance,
            ];
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
