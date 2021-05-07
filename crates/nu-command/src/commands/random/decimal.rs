use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Range, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

pub struct SubCommand;

pub struct DecimalArgs {
    range: Option<Tagged<Range>>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("random decimal").optional("range", SyntaxShape::Range, "Range of values")
    }

    fn usage(&self) -> &str {
        "Generate a random decimal within a range [min..max]"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        decimal(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a default decimal value between 0 and 1",
                example: "random decimal",
                result: None,
            },
            Example {
                description: "Generate a random decimal less than or equal to 500",
                example: "random decimal ..500",
                result: None,
            },
            Example {
                description: "Generate a random decimal greater than or equal to 100000",
                example: "random decimal 100000..",
                result: None,
            },
            Example {
                description: "Generate a random decimal between 1.0 and 1.1",
                example: "random decimal 1.0..1.1",
                result: None,
            },
        ]
    }
}

pub fn decimal(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    let cmd_args = DecimalArgs {
        range: args.opt(0)?,
    };

    let (min, max) = if let Some(range) = &cmd_args.range {
        (range.item.min_f64()?, range.item.max_f64()?)
    } else {
        (0.0, 1.0)
    };

    match min.partial_cmp(&max) {
        Some(Ordering::Greater) => Err(ShellError::labeled_error(
            format!("Invalid range {}..{}", min, max),
            "expected a valid range",
            cmd_args
                .range
                .expect("Unexpected ordering error in random decimal")
                .span(),
        )),
        Some(Ordering::Equal) => Ok(OutputStream::one(UntaggedValue::decimal_from_float(
            min,
            Span::new(64, 64),
        ))),
        _ => {
            let mut thread_rng = thread_rng();
            let result: f64 = thread_rng.gen_range(min, max);

            Ok(OutputStream::one(UntaggedValue::decimal_from_float(
                result,
                Span::new(64, 64),
            )))
        }
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
