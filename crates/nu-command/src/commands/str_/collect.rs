use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("str collect").desc(self.usage()).optional(
            "separator",
            SyntaxShape::String,
            "the separator to put between the different values",
        )
    }

    fn usage(&self) -> &str {
        "collects a list of strings into a string"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collect(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Collect a list of string",
            example: "echo ['a' 'b' 'c'] | str collect",
            result: Some(vec![Value::from("abc")]),
        }]
    }
}

pub fn collect(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    //let (SubCommandArgs { separator }, input) = args.process()?;
    let args = args.evaluate_once()?;
    let separator: Option<Result<Tagged<String>, ShellError>> = args.opt(0);
    let input = args.input;
    let separator = if let Some(separator) = separator {
        separator?.item
    } else {
        "".into()
    };

    let strings: Vec<Result<String, ShellError>> = input.map(|value| value.as_string()).collect();
    let strings: Result<Vec<_>, _> = strings.into_iter().collect::<Result<_, _>>();

    match strings {
        Ok(strings) => {
            let output = strings.join(&separator);

            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(tag),
            )))
        }
        Err(err) => match err.error {
            nu_errors::ProximateShellError::TypeError { actual, .. } => {
                if let Some(item) = actual.item {
                    Err(ShellError::labeled_error_with_secondary(
                        "could not convert to string",
                        format!("tried to convert '{}' in input to a string", item),
                        tag.span,
                        format!("'{}' value originated here", item),
                        actual.span,
                    ))
                } else {
                    Err(ShellError::labeled_error_with_secondary(
                        "could not convert to string",
                        "failed to convert input to strings",
                        tag.span,
                        "non-string found here",
                        actual.span,
                    ))
                }
            }
            _ => Err(err),
        },
    }
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
