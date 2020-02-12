use crate::commands::PerItemCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Primitive, ReturnSuccess, UntaggedValue, Value};

pub struct Calc;

impl PerItemCommand for Calc {
    fn name(&self) -> &str {
        "calc"
    }

    fn usage(&self) -> &str {
        "Parse a math expression into a number"
    }

    fn run(
        &self,
        _call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        calc(input, raw_args)
    }
}

fn calc(input: Value, args: &RawCommandArgs) -> Result<OutputStream, ShellError> {
    let name_span = &args.call_info.name_tag.span;

    let output = if let Ok(string) = input.as_string() {
        match parse(&string, &input.tag) {
            Ok(value) => ReturnSuccess::value(value),
            Err(err) => Err(ShellError::labeled_error(
                "Calulation error",
                err,
                &input.tag.span,
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "Expected a string from pipeline",
            "requires string input",
            name_span,
        ))
    };

    Ok(vec![output].into())
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
