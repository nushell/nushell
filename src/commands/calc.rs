use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Calc;

impl WholeStreamCommand for Calc {
    fn name(&self) -> &str {
        "calc"
    }

    fn signature(&self) -> Signature {
        Signature::build("calc").required(
            "math-expression",
            SyntaxShape::String,
            "the expression to parse into a number",
        )
    }

    fn usage(&self) -> &str {
        "Parse a math expression into a number"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        calc(args, registry)
    }
}

fn calc(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = &args.call_info.name_tag;
    let name_span = tag.span;
    let input = match args.args.nth(0) {
        Some(s) => Ok(s),
        None => Err(ShellError::labeled_error(
            "Expected a string argument",
            "requires string input",
            name_span,
        )),
    };
    let input = input?;

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
            "Expected a string argument",
            "requires string input",
            name_span,
        ))
    };

    Ok(vec![output].into())
}

pub fn parse(math_expression: &str, tag: impl Into<Tag>) -> Result<Value, String> {
    let num = meval::eval_str(math_expression);
    match num {
        Ok(num) => Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag)),
        Err(error) => Err(error.to_string()),
    }
}
