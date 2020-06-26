use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    Primitive,
    ReturnSuccess,
    Signature,
    UntaggedValue,
    Value, //  SyntaxShape, UnspannedPathMember, , Primitive,
};
use num_bigint::{BigInt, ToBigInt};
use num_format::{Locale, SystemLocale, ToFormattedString};

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
        registry: &CommandRegistry,
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

fn format_for_locale(int: BigInt) -> UntaggedValue {
    UntaggedValue::string(match SystemLocale::default() {
        Ok(locale) => int.to_formatted_string(&locale),
        Err(err) => int.to_formatted_string(&Locale::en), // TODO acceptable?
    })
}

// TODO If you're using the with-system-locale feature and you're on Windows, Clang 3.9 or higher is also required.
async fn human(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    // TODO commandargs .process or .evaluate_once
    Ok(args
        .input
        .map(move |val| {
            println!("{:#?}", val);
            let pass_through = ReturnSuccess::Value(val.clone());  // TODO more clever way around this
            match val.value {
                // Value {
                    // value: UntaggedValue::Primitive(prim),
                    // ..
                // }
                UntaggedValue::Primitive(prim) => match prim {
                    Primitive::Int(int) => ReturnSuccess::value(format_for_locale(int)),
                    // TODO expect
                    // UntaggedValue::string((*decimal).to_bigint().expect("BigDecimal failed to convert to Bigint").to_formatted_string(&locale)),
                    // TODO clone/dereference iffy?
                    Primitive::Decimal(decimal) if decimal.is_integer() => ReturnSuccess::value(
                        format_for_locale(decimal.to_bigint().expect("BigDecimal failed to convert to Bigint"))
                    ),
                    Primitive::Decimal(bd) => {
                        // let inpf = bd.to_formatted_string(&Locale::en);
                        println!("{:#?}", bd);
                        let bdr = bd.with_prec(3);
                        println!("{:#?}", bdr);
                        let bds = bd.with_scale(0);
                        println!("{:#?}", bds);
                        let (bis, es) = bds.into_bigint_and_exponent();
                        println!("{} {}", bis, es);
                        let (bi, e) = bd.as_bigint_and_exponent();
                        println!(
                            "{} {:?} {} = {:#?} {:#?}",
                            bd.digits(),
                            bd.to_i64(),
                            bi,
                            bi,
                            e
                        );
                        // println!("{}", bi.to_formatted_string(&Locale::en));

                        Err(ShellError::unimplemented("human for non-int decimals"))
                    },
                    _ => Ok(pass_through),
                },
                _ => Err(ShellError::unimplemented("human for non-primitives")),
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
