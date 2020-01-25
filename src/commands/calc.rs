use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue};

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

pub fn calc(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let arg = &args.parts().1.positional.unwrap_or_else(|| {
        vec![UntaggedValue::from(Primitive::String("0".to_string())).into_untagged_value()]
    })[0];

    let math_expression = arg.as_string().unwrap_or_else(|_| "0".to_string());
    let value = UntaggedValue::from(Primitive::from(
        meval::eval_str(&math_expression).unwrap_or_default(),
    ))
    .into_untagged_value();
    let output = vec![value];

    Ok(OutputStream::from(output))
}
