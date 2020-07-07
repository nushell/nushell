use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use num_bigint::{BigInt, BigUint, ToBigInt};
use num_format::{SystemLocale, ToFormattedString};
use num_traits::{Pow, Signed};

pub struct SubCommand;

#[derive(Deserialize)]
struct Arguments {
    decimals: Option<Tagged<u64>>,
    #[serde(rename(deserialize = "group-digits"))]
    group_digits: bool,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str from"
    }

    fn signature(&self) -> Signature {
        Signature::build("str from")
            .named(
                "decimals",
                SyntaxShape::Int,
                "decimal digits to which to round",
                Some('d'),
            )
            .switch(
                "group-digits",
                "group digits according to system localization",
                Some('g'),
            )
    }

    fn usage(&self) -> &str {
        "Converts numeric types to strings. Trims trailing zeros unless decimals parameter is specified."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_str(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "round to nearest integer",
                example: "= 1.8 | str from -d 0",
                result: Some(vec![UntaggedValue::string("2").into_untagged_value()]),
            },
            Example {
                description: "format large number with localized digit grouping",
                example: "= 1000000 | str from -g", // TODO localization
                result: Some(vec![
                    UntaggedValue::string("1,000,000").into_untagged_value()
                ]),
            },
        ]
    }
}

fn format_decimal(
    mut decimal: BigDecimal,
    digits: Option<u64>,
    locale: Option<&SystemLocale>,
) -> String {
    decimal = match digits {
        None => decimal,
        Some(n) => round_decimal(&decimal, n),
    };

    if decimal.is_integer() {
        // TODO append zeros?
        let int = decimal
            .to_bigint()
            .expect("integer BigDecimal should convert to BigInt");
        return match locale {
            None => int.to_string(),
            Some(loc) => int.to_formatted_string(loc),
        };
    }

    let (int, exp) = decimal.as_bigint_and_exponent();
    let factor = BigInt::from(10).pow(BigUint::from(exp as u64));
    let int_part = &int / &factor;
    let dec_part = (&int % &factor)
        .abs()
        .to_biguint()
        .expect("BigInt::abs should always produce positive signed BigInt and thus BigUInt")
        .to_str_radix(10);

    let mut dec_str = String::from("");
    let mut backlog = String::from("");
    if let Some(n) = digits {
        dec_str = dec_part.chars().take(n as usize).collect();
    } else {
        dec_part.chars().for_each(|ch| match ch {
            '0' => backlog.push('0'),
            other => {
                dec_str.push_str(backlog.as_str());
                backlog = String::from("");
                dec_str.push(other);
            }
        });
    }

    let (int_part_str, sep) = match locale {
        None => (int_part.to_string(), "."),
        Some(loc) => (int_part.to_formatted_string(loc), loc.decimal()),
    };

    format!("{}{}{}", int_part_str, sep, dec_str)
}

fn round_decimal(decimal: &BigDecimal, mut digits: u64) -> BigDecimal {
    let mut mag = decimal.clone();
    while mag >= BigDecimal::from(1) {
        mag = mag / 10;
        digits += 1;
    }

    decimal.with_prec(digits)
}

async fn to_str(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let (
        Arguments {
            decimals,
            group_digits,
        },
        input,
    ) = args.process(&registry.clone()).await?;
    let locale = match SystemLocale::default() {
        Ok(locale) => locale,
        Err(_) => {
            return Err(ShellError::unexpected(
                "num-format failed to load system locale",
            ))
        }
    };

    Ok(input
        .map(move |val| {
            ReturnSuccess::value(match val.value {
                UntaggedValue::Primitive(prim) => UntaggedValue::string(match prim {
                    Primitive::Int(int) => {
                        if group_digits {
                            int.to_formatted_string(&locale)
                        } else {
                            int.to_string()
                        }
                    }
                    Primitive::Decimal(dec) => format_decimal(
                        dec,
                        decimals.as_ref().map(|tagged| tagged.item),
                        if group_digits { Some(&locale) } else { None },
                    ),
                    other => other.into_string(val.tag.span)?,
                }),
                other => other,
            })
        })
        .to_output_stream())
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
