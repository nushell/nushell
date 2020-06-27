use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use num_bigint::{BigInt, ToBigInt};
use num_format::{SystemLocale, ToFormattedString};
use num_traits::Signed;
use std::convert::TryInto;

pub struct Human;

#[async_trait]
impl WholeStreamCommand for Human {
    fn name(&self) -> &str {
        "human"
    }

    fn signature(&self) -> Signature {
        Signature::build("human")
    }

    fn usage(&self) -> &str {
        "Formats (TODO decimal) numbers for human sensibilities."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry, // TODO use for flags etc
    ) -> Result<OutputStream, ShellError> {
        human(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "TODO",
                example: "= 5 / 7 | human -p 3",
                result: Some(vec![Value::from("0.714")]),
            },
            Example {
                description: "TODO",
                example: "echo '10^10' | calc | human",
                result: Some(vec![Value::from("10,000,000,000")]), //  TODO localization
            },
        ]
    }
}

fn format_decimal(decimal: BigDecimal, locale: SystemLocale) -> String {
    let (int, exp) = decimal.as_bigint_and_exponent();

    // TODO okay to assert?
    assert!(exp > 0);

    let uexp = exp as u64;
    let factor = 10_u64.pow(
        uexp.try_into()
            .expect("coercing u64 (from i64) BigDecimal exponent to u32"),
    );
    let int_part = &int / factor;
    let dec_part = (&int % factor)
        .abs()
        .to_biguint()
        .expect("abs should produce positive signed bigint");
    let dec_str: String = dec_part
        .to_str_radix(10)
        .chars()
        .rev()
        .fold(String::from(""), |acc, ch| match (acc, ch) {
            (a, '0') if a == String::from("") => a,
            (mut a, c) => {
                a.push(c);
                a
            }
        })
        .chars()
        .rev()
        .collect(); // TODO acceptable?

    format!(
        "{}{}{}",
        int_part.to_formatted_string(&locale),
        locale.decimal(),
        dec_str
    )
}

fn format_int(int: BigInt, locale: SystemLocale) -> String {
    int.to_formatted_string(&locale)
    // match SystemLocale::default() {
    //     Ok(locale) => ,
    //     Err(_) => int.to_formatted_string(&Locale::en), // TODO acceptable?
    // }
}

fn format_value(val: UntaggedValue) -> Result<UntaggedValue, &'static str> {
    if let UntaggedValue::Primitive(prim) = val.clone() {
        let locale = match SystemLocale::default() {
            Ok(locale) => locale,
            Err(_) => SystemLocale::from_name("POSIX")
                .expect("num-format should load locale from known str"), // TODO acceptable? en_US?
        };
        Ok(match prim {
            Primitive::Int(int) => UntaggedValue::string(format_int(int, locale)),
            // TODO expect
            // UntaggedValue::string((*decimal).to_bigint().expect("BigDecimal failed to convert to Bigint").to_formatted_string(&locale)),
            // TODO clone/dereference iffy?
            Primitive::Decimal(decimal) if decimal.is_integer() => {
                UntaggedValue::string(format_int(
                    match decimal.to_bigint() {
                        Some(bi) => bi,
                        None => return Err("int BigDecimal failed to convert to Bigint"),
                    },
                    locale,
                ))
            }
            Primitive::Decimal(bd) => UntaggedValue::string(format_decimal(bd, locale)),
            _ => val,
        })
    } else {
        Ok(val) // Err(ShellError::unimplemented("human for non-primitives")),
    }
}

// TODO If you're using the with-system-locale feature and you're on Windows, Clang 3.9 or higher is also required.
async fn human(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    // TODO commandargs .process or .evaluate_once
    Ok(args
        .input
        .map(|val| {
            // let pass_through = ReturnSuccess::Value(val.clone()); // TODO more clever way around this
            // let v = UntaggedValue::from(val.clone());
            // println!("{:#?}", v);
            match format_value(UntaggedValue::from(val)) {
                Ok(v) => ReturnSuccess::value(v),
                Err(s) => return Err(ShellError::unexpected(s)),
            }
        })
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
