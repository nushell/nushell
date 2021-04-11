use crate::prelude::*;
use bigdecimal::Zero;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{
    Primitive, Range, RangeInclusion, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct Echo;

impl WholeStreamCommand for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        echo(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
                result: Some(vec![Value::from("hello")]),
            },
            Example {
                description: "Print the value of the special '$nu' variable",
                example: "echo $nu",
                result: None,
            },
        ]
    }
}

fn echo(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    let rest: Vec<Value> = args.rest(0)?;

    let stream = rest.into_iter().map(|i| match i.as_string() {
        Ok(s) => OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(s).into_value(i.tag.clone()),
        ))),
        _ => match i {
            Value {
                value: UntaggedValue::Table(table),
                ..
            } => table
                .into_iter()
                .map(ReturnSuccess::value)
                .to_output_stream(),
            Value {
                value: UntaggedValue::Primitive(Primitive::Range(range)),
                tag,
            } => RangeIterator::new(*range, tag).to_output_stream(),
            x => OutputStream::one(Ok(ReturnSuccess::Value(x))),
        },
    });

    Ok(stream.flatten().to_output_stream())
}

struct RangeIterator {
    curr: UntaggedValue,
    end: UntaggedValue,
    tag: Tag,
    is_end_inclusive: bool,
    moves_up: bool,
    one: UntaggedValue,
    negative_one: UntaggedValue,
}

impl RangeIterator {
    pub fn new(range: Range, tag: Tag) -> RangeIterator {
        let start = match range.from.0.item {
            Primitive::Nothing => Primitive::Int(0.into()),
            x => x,
        };

        let end = match range.to.0.item {
            Primitive::Nothing => Primitive::Int(u64::MAX.into()),
            x => x,
        };

        RangeIterator {
            moves_up: start <= end,
            curr: UntaggedValue::Primitive(start),
            end: UntaggedValue::Primitive(end),
            tag,
            is_end_inclusive: matches!(range.to.1, RangeInclusion::Inclusive),
            one: UntaggedValue::int(1),
            negative_one: UntaggedValue::int(-1),
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Result<ReturnSuccess, ShellError>;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering;

        let ordering = if self.end == UntaggedValue::Primitive(Primitive::Nothing) {
            Ordering::Less
        } else {
            match (&self.curr, &self.end) {
                (
                    UntaggedValue::Primitive(Primitive::Int(x)),
                    UntaggedValue::Primitive(Primitive::Int(y)),
                ) => x.cmp(y),
                (
                    UntaggedValue::Primitive(Primitive::Decimal(x)),
                    UntaggedValue::Primitive(Primitive::Decimal(y)),
                ) => x.cmp(y),
                (
                    UntaggedValue::Primitive(Primitive::Decimal(x)),
                    UntaggedValue::Primitive(Primitive::Int(y)),
                ) => x.cmp(&(BigDecimal::zero() + y)),
                (
                    UntaggedValue::Primitive(Primitive::Int(x)),
                    UntaggedValue::Primitive(Primitive::Decimal(y)),
                ) => (BigDecimal::zero() + x).cmp(y),
                _ => {
                    return Some(Err(ShellError::labeled_error(
                        "Cannot create range",
                        "unsupported range",
                        self.tag.span,
                    )))
                }
            }
        };

        if self.moves_up
            && (ordering == Ordering::Less || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value = nu_data::value::compute_values(Operator::Plus, &self.curr, &self.one);

            let mut next = match next_value {
                Ok(result) => result,

                Err((left_type, right_type)) => {
                    return Some(Err(ShellError::coerce_error(
                        left_type.spanned(self.tag.span),
                        right_type.spanned(self.tag.span),
                    )));
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(ReturnSuccess::value(next.into_value(self.tag.clone())))
        } else if !self.moves_up
            && (ordering == Ordering::Greater
                || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value =
                nu_data::value::compute_values(Operator::Plus, &self.curr, &self.negative_one);

            let mut next = match next_value {
                Ok(result) => result,
                Err((left_type, right_type)) => {
                    return Some(Err(ShellError::coerce_error(
                        left_type.spanned(self.tag.span),
                        right_type.spanned(self.tag.span),
                    )));
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(ReturnSuccess::value(next.into_value(self.tag.clone())))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Echo;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Echo {})
    }
}
