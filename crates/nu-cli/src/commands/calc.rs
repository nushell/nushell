use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};

pub struct Calc;

#[derive(Deserialize)]
pub struct CalcArgs {}

impl WholeStreamCommand for Calc {
    fn name(&self) -> &str {
        "calc"
    }

    fn usage(&self) -> &str {
        "Parse a math expression into a number"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, calc)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Calculate math in the pipeline",
            example: "echo '10 / 4' | calc",
        }]
    }
}

pub fn calc(
    _: CalcArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
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
                    name.clone(),
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
