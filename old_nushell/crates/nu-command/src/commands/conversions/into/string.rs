use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_bigint::{BigInt, BigUint, ToBigInt};
// TODO num_format::SystemLocale once platform-specific dependencies are stable (see Cargo.toml)
use nu_data::base::shape::InlineShape;
use num_format::Locale;
use num_traits::{Pow, Signed};
use std::iter;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "into string"
    }

    fn signature(&self) -> Signature {
        Signature::build("into string")
            .rest(
                "rest",
                SyntaxShape::ColumnPath,
                "column paths to convert to string (for table input)",
            )
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        into_string(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert decimal to string and round to nearest integer",
                example: "echo 1.7 | into string -d 0",
                result: Some(vec![UntaggedValue::string("2").into_untagged_value()]),
            },
            Example {
                description: "convert decimal to string",
                example: "echo 4.3 | into string",
                result: Some(vec![UntaggedValue::string("4.3").into_untagged_value()]),
            },
            Example {
                description: "convert string to string",
                example: "echo '1234' | into string",
                result: Some(vec![UntaggedValue::string("1234").into_untagged_value()]),
            },
            Example {
                description: "convert boolean to string",
                example: "echo $true | into string",
                result: Some(vec![UntaggedValue::string("true").into_untagged_value()]),
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

fn into_string(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let decimals: Option<Tagged<u64>> = args.get_flag("decimals")?;
    let column_paths: Vec<ColumnPath> = args.rest(0)?;

    let digits = decimals.as_ref().map(|tagged| tagged.item);
    let group_digits = false;

    Ok(args
        .input
        .map(move |v| {
            if column_paths.is_empty() {
                action(&v, v.tag(), digits, group_digits)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag(), digits, group_digits)),
                    )?;
                }

                Ok(ret)
            }
        })
        .into_input_stream())
}

pub fn action(
    input: &Value,
    tag: impl Into<Tag>,
    digits: Option<u64>,
    group_digits: bool,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(prim) => Ok(UntaggedValue::string(match prim {
            Primitive::Int(int) => {
                if group_digits {
                    format_int(*int) // int.to_formatted_string(*locale)
                } else {
                    int.to_string()
                }
            }
            Primitive::BigInt(int) => {
                if group_digits {
                    format_bigint(int) // int.to_formatted_string(*locale)
                } else {
                    int.to_string()
                }
            }
            Primitive::Decimal(dec) => format_decimal(dec.clone(), digits, group_digits),
            Primitive::String(a_string) => a_string.to_string(),
            Primitive::Boolean(a_bool) => a_bool.to_string(),
            Primitive::Date(a_date) => a_date.format("%c").to_string(),
            Primitive::FilePath(a_filepath) => a_filepath.as_path().display().to_string(),
            Primitive::Filesize(a_filesize) => {
                let byte_string = InlineShape::format_bytes(*a_filesize, None);
                byte_string.1
            }
            Primitive::Nothing => "nothing".to_string(),
            _ => {
                return Err(ShellError::unimplemented(&format!(
                    "into string for primitive: {:?}",
                    prim
                )))
            }
        })
        .into_value(tag)),
        UntaggedValue::Row(_) => Err(ShellError::labeled_error(
            "specify column to use 'into string'",
            "found table",
            input.tag.clone(),
        )),
        UntaggedValue::Table(_) => Err(ShellError::unimplemented("into string for table")),
        _ => Err(ShellError::unimplemented("into string for non-primitive")),
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
        let int = decimal
            .to_bigint()
            .expect("integer BigDecimal should convert to BigInt");
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
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
