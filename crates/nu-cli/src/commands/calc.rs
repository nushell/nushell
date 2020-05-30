use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};

pub struct Calc;

#[async_trait]
impl WholeStreamCommand for Calc {
    fn name(&self) -> &str {
        "calc"
    }

    fn usage(&self) -> &str {
        "Parse a math expression into a number"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        calc(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculate math in the pipeline",
            example: "echo '10 / 4' | calc",
            result: Some(vec![UntaggedValue::decimal(2.5).into()]),
        }]
    }
}

pub async fn calc(
    args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let name = args.call_info.name_tag.span;

    Ok(input
        .map(move |input| {
            if let Ok(string) = input.as_string() {
                match parse(&string, &input.tag) {
                    Ok(value) => ReturnSuccess::value(value),
                    Err(err) => Err(ShellError::labeled_error(
                        "Calculation error",
                        err,
                        &input.tag.span,
                    )),
                }
            } else {
                Err(ShellError::labeled_error(
                    "Expected a string from pipeline",
                    "requires string input",
                    name,
                ))
            }
        })
        .to_output_stream())
}

pub fn parse(math_expression: &str, tag: impl Into<Tag>) -> Result<Value, String> {
    use std::f64;
    let num = meval::eval_str(math_expression);
    match num {
        Ok(num) => {
            if num == f64::INFINITY || num == f64::NEG_INFINITY {
                return Err(String::from("cannot represent result"));
            }
            Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag))
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::Calc;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Calc {})
    }
}
