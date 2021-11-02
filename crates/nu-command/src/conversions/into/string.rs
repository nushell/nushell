use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use bigdecimal::{BigDecimal, FromPrimitive};
use num_bigint::{BigInt, BigUint};
use num_format::Locale;
use num_traits::{Pow, Signed};
use std::iter;
// TODO num_format::SystemLocale once platform-specific dependencies are stable (see Cargo.toml)

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into string"
    }

    fn signature(&self) -> Signature {
        Signature::build("into string")
            // FIXME - need to support column paths
            // .rest(
            //     "rest",
            //     SyntaxShape::ColumnPaths(),
            //     "column paths to convert to string (for table input)",
            // )
            .named(
                "decimals",
                SyntaxShape::Int,
                "decimal digits to which to round",
                Some('d'),
            )
    }

    fn usage(&self) -> &str {
        "Convert value to string"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        string_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert decimal to string and round to nearest integer",
                example: "1.7 | into string -d 0",
                result: Some(Value::String {
                    val: "2".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert decimal to string",
                example: "1.7 | into string -d 1",
                result: Some(Value::String {
                    val: "1.7".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert decimal to string and limit to 2 decimals",
                example: "1.734 | into string -d 2",
                result: Some(Value::String {
                    val: "1.73".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "try to convert decimal to string and provide negative decimal points",
                example: "1.734 | into string -d -2",
                result: None,
                // FIXME
                // result: Some(Value::Error {
                //     error: ShellError::UnsupportedInput(
                //         String::from("Cannot accept negative integers for decimals arguments"),
                //         Span::unknown(),
                //     ),
                // }),
            },
            Example {
                description: "convert decimal to string",
                example: "4.3 | into string",
                result: Some(Value::String {
                    val: "4.3".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert string to string",
                example: "'1234' | into string",
                result: Some(Value::String {
                    val: "1234".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert boolean to string",
                example: "$true | into string",
                result: Some(Value::String {
                    val: "true".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert date to string",
                example: "date now | into string",
                result: None,
            },
            Example {
                description: "convert filepath to string",
                example: "ls Cargo.toml | get name | into string",
                result: None,
            },
            Example {
                description: "convert filesize to string",
                example: "ls Cargo.toml | get size | into string",
                result: None,
            },
        ]
    }
}

fn string_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, ShellError> {
    let decimals = call.has_flag("decimals");
    let head = call.head;
    let decimals_value: Option<i64> = call.get_flag(engine_state, stack, "decimals")?;

    if decimals && decimals_value.is_some() && decimals_value.unwrap().is_negative() {
        return Err(ShellError::UnsupportedInput(
            "Cannot accept negative integers for decimals arguments".to_string(),
            head,
        ));
    }

    input.map(
        move |v| action(v, head, decimals, decimals_value, false),
        engine_state.ctrlc.clone(),
    )
}

pub fn action(
    input: Value,
    head: Span,
    decimals: bool,
    digits: Option<i64>,
    group_digits: bool,
) -> Value {
    match input {
        Value::Int { val, span: _ } => {
            let res = if group_digits {
                format_int(val) // int.to_formatted_string(*locale)
            } else {
                val.to_string()
            };

            Value::String {
                val: res,
                span: head,
            }
        }
        Value::Float { val, span: _ } => {
            if decimals {
                let dec = BigDecimal::from_f64(val);
                let decimal_value = digits.unwrap() as u64;
                match dec {
                    Some(x) => Value::String {
                        val: format_decimal(x, Some(decimal_value), group_digits),
                        span: head,
                    },
                    None => Value::Error {
                        error: ShellError::CantConvert(
                            format!("cannot convert {} to BigDecimal", val),
                            head,
                        ),
                    },
                }
            } else {
                Value::String {
                    val: val.to_string(),
                    span: head,
                }
            }
        }
        // We do not seem to have BigInt at the moment as a Value Type
        // Value::BigInt { val, span } => {
        //     let res = if group_digits {
        //         format_bigint(val) // int.to_formatted_string(*locale)
        //     } else {
        //         int.to_string()
        //     };

        //     Value::String {
        //         val: res,
        //         span: head,
        //     }
        //     .into_pipeline_data()
        // }
        Value::Bool { val, span: _ } => Value::String {
            val: val.to_string(),
            span: head,
        },

        Value::Date { val, span: _ } => Value::String {
            val: val.format("%c").to_string(),
            span: head,
        },

        Value::String { val, span: _ } => Value::String { val, span: head },

        // FIXME - we do not have a FilePath type anymore. Do we need to support this?
        // Value::FilePath(a_filepath) => a_filepath.as_path().display().to_string(),
        Value::Filesize { val: _, span: _ } => Value::String {
            val: input.into_string(),
            span: head,
        },
        Value::Nothing { span: _ } => Value::String {
            val: "nothing".to_string(),
            span: head,
        },
        Value::Record {
            cols: _,
            vals: _,
            span: _,
        } => Value::Error {
            error: ShellError::UnsupportedInput(
                "Cannot convert Record into string".to_string(),
                head,
            ),
        },

        _ => Value::Error {
            error: ShellError::CantConvert(
                String::from(" into string. Probably this type is not supported yet"),
                head,
            ),
        },
    }
}
fn format_int(int: i64) -> String {
    int.to_string()

    // TODO once platform-specific dependencies are stable (see Cargo.toml)
    // #[cfg(windows)]
    // {
    //     int.to_formatted_string(&Locale::en)
    // }
    // #[cfg(not(windows))]
    // {
    //     match SystemLocale::default() {
    //         Ok(locale) => int.to_formatted_string(&locale),
    //         Err(_) => int.to_formatted_string(&Locale::en),
    //     }
    // }
}

fn format_bigint(int: &BigInt) -> String {
    int.to_string()

    // TODO once platform-specific dependencies are stable (see Cargo.toml)
    // #[cfg(windows)]
    // {
    //     int.to_formatted_string(&Locale::en)
    // }
    // #[cfg(not(windows))]
    // {
    //     match SystemLocale::default() {
    //         Ok(locale) => int.to_formatted_string(&locale),
    //         Err(_) => int.to_formatted_string(&Locale::en),
    //     }
    // }
}

fn format_decimal(mut decimal: BigDecimal, digits: Option<u64>, group_digits: bool) -> String {
    if let Some(n) = digits {
        decimal = round_decimal(&decimal, n)
    }

    if decimal.is_integer() && (digits.is_none() || digits == Some(0)) {
        let int = decimal.as_bigint_and_exponent().0;
        // .expect("integer BigDecimal should convert to BigInt");
        return if group_digits {
            int.to_string()
        } else {
            format_bigint(&int)
        };
    }

    let (int, exp) = decimal.as_bigint_and_exponent();
    let factor = BigInt::from(10).pow(BigUint::from(exp as u64)); // exp > 0 for non-int decimal
    let int_part = &int / &factor;
    let dec_part = (&int % &factor)
        .abs()
        .to_biguint()
        .expect("BigInt::abs should always produce positive signed BigInt and thus BigUInt")
        .to_str_radix(10);

    let dec_str = if let Some(n) = digits {
        dec_part
            .chars()
            .chain(iter::repeat('0'))
            .take(n as usize)
            .collect()
    } else {
        String::from(dec_part.trim_end_matches('0'))
    };

    let format_default_loc = |int_part: BigInt| {
        let loc = Locale::en;
        //TODO: when num_format is available for recent bigint, replace this with the locale-based format
        let (int_str, sep) = (int_part.to_string(), String::from(loc.decimal()));

        format!("{}{}{}", int_str, sep, dec_str)
    };

    format_default_loc(int_part)

    // TODO once platform-specific dependencies are stable (see Cargo.toml)
    // #[cfg(windows)]
    // {
    //     format_default_loc(int_part)
    // }
    // #[cfg(not(windows))]
    // {
    //     match SystemLocale::default() {
    //         Ok(sys_loc) => {
    //             let int_str = int_part.to_formatted_string(&sys_loc);
    //             let sep = String::from(sys_loc.decimal());
    //             format!("{}{}{}", int_str, sep, dec_str)
    //         }
    //         Err(_) => format_default_loc(int_part),
    //     }
    // }
}

fn round_decimal(decimal: &BigDecimal, mut digits: u64) -> BigDecimal {
    let mut mag = decimal.clone();
    while mag >= BigDecimal::from(1) {
        mag = mag / 10;
        digits += 1;
    }

    decimal.with_prec(digits)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
