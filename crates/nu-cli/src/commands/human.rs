use crate::commands::precision::as_rounded_decimal;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_bigint::{BigInt, BigUint, ToBigInt};
use num_format::{SystemLocale, ToFormattedString};
use num_traits::{Pow, Signed};

pub struct Human;

#[derive(Deserialize)]
pub struct HumanArgs {
    precision: Option<Tagged<u64>>,
}

#[async_trait]
impl WholeStreamCommand for Human {
    fn name(&self) -> &str {
        "human"
    }

    fn signature(&self) -> Signature {
        Signature::build("human").named(
            "precision",
            SyntaxShape::Int,
            "number of decimal digits to which to round",
            Some('p'),
        )
    }

    fn usage(&self) -> &str {
        "Formats large numbers and decimals per system localization."
    }
    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        human(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Format numbers with localized digit grouping and decimal separator, rounded to decimal digit",
                example: "= 10323.78 | human -p 1",
                result: Some(vec![Value::from("10,323.8")]), //  TODO localization
            },
        ]
    }
}

fn format_decimal(decimal: BigDecimal, locale: SystemLocale) -> String {
    let (int, exp) = decimal.as_bigint_and_exponent();

    if decimal.is_integer() {
        return decimal
            .to_bigint()
            .expect("integer BigDecimal should convert to BigInt")
            .to_formatted_string(&locale);
    }

    let factor = BigInt::from(10).pow(BigUint::from(exp as u64));
    let int_part = &int / &factor;
    let dec_part = (&int % &factor)
        .abs()
        .to_biguint()
        .expect("BigInt::abs should always produce positive signed BigInt and thus BigUInt");
    let dec_str: String = dec_part
        .to_str_radix(10)
        .chars()
        .rev()
        .fold(String::from(""), |acc, ch| match (acc, ch) {
            // strip trailing zeros
            (a, '0') if a == String::from("") => a,
            (mut a, c) => {
                a.push(c);
                a
            }
        })
        .chars()
        .rev()
        .collect();

    format!(
        "{}{}{}",
        int_part.to_formatted_string(&locale),
        locale.decimal(),
        dec_str
    )
}

fn format_value(val: UntaggedValue, prec_digits: Option<u64>) -> Result<UntaggedValue, ShellError> {
    if let UntaggedValue::Primitive(prim) = val.clone() {
        let locale = match SystemLocale::default() {
            Ok(locale) => locale,
            Err(_) => {
                return Err(ShellError::unexpected(
                    "num-format failed to load system locale",
                ))
            }
        };
        Ok(match prim {
            Primitive::Int(int) => UntaggedValue::string(int.to_formatted_string(&locale)),
            Primitive::Decimal(dec) => {
                let decimal = match prec_digits {
                    None => dec,
                    Some(digits) => as_rounded_decimal(&dec, digits),
                };
                UntaggedValue::string(format_decimal(decimal, locale))
            }
            _ => val,
        })
    } else {
        Ok(val)
    }
}

// TODO If you're using the with-system-locale feature and you're on Windows, Clang 3.9 or higher is also required.
async fn human(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let (args, input): (HumanArgs, _) = args.process(&registry.clone()).await?;
    let prec_digits = args.precision.map(|tagged| tagged.item);

    Ok(input
        .map(
            move |val| match format_value(UntaggedValue::from(val), prec_digits) {
                Ok(v) => ReturnSuccess::value(v),
                Err(s) => return Err(s),
            },
        )
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Human;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Human {})
    }
}
