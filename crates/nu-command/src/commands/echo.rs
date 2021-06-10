use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{Primitive, Range, RangeInclusion, Signature, SyntaxShape, UntaggedValue, Value};

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

    fn run(&self, args: CommandArgs) -> Result<InputStream, ShellError> {
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

pub fn expand_value_to_stream(v: Value) -> InputStream {
    match v {
        Value {
            value: UntaggedValue::Table(table),
            ..
        } => InputStream::from_stream(table.into_iter()),
        Value {
            value: UntaggedValue::Primitive(Primitive::Range(range)),
            tag,
        } => InputStream::from_stream(RangeIterator::new(*range, tag)),
        x => InputStream::one(x),
    }
}

fn echo(args: CommandArgs) -> Result<InputStream, ShellError> {
    let rest: Vec<Value> = args.rest(0)?;

    let stream = rest.into_iter().map(|i| match i.as_string() {
        Ok(s) => InputStream::one(UntaggedValue::string(s).into_value(i.tag)),
        _ => expand_value_to_stream(i),
    });

    Ok(InputStream::from_stream(stream.flatten()))
}

struct RangeIterator {
    curr: UntaggedValue,
    end: UntaggedValue,
    tag: Tag,
    is_end_inclusive: bool,
    moves_up: bool,
    one: UntaggedValue,
    negative_one: UntaggedValue,
    done: bool,
}

impl RangeIterator {
    pub fn new(range: Range, tag: Tag) -> RangeIterator {
        let start = match range.from.0.item {
            Primitive::Nothing => Primitive::Int(0.into()),
            x => x,
        };

        let end = match range.to.0.item {
            Primitive::Nothing => Primitive::Int(i64::MAX),
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
            done: false,
        }
    }
}

impl Iterator for RangeIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering;
        if self.done {
            return None;
        }

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
                ) => x.cmp(&(BigDecimal::from(*y))),
                (
                    UntaggedValue::Primitive(Primitive::Int(x)),
                    UntaggedValue::Primitive(Primitive::Decimal(y)),
                ) => (BigDecimal::from(*x)).cmp(y),
                _ => {
                    self.done = true;
                    return Some(
                        UntaggedValue::Error(ShellError::labeled_error(
                            "Cannot create range",
                            "unsupported range",
                            self.tag.span,
                        ))
                        .into_untagged_value(),
                    );
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
                    self.done = true;
                    return Some(
                        UntaggedValue::Error(ShellError::coerce_error(
                            left_type.spanned(self.tag.span),
                            right_type.spanned(self.tag.span),
                        ))
                        .into_untagged_value(),
                    );
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next.into_value(self.tag.clone()))
        } else if !self.moves_up
            && (ordering == Ordering::Greater
                || self.is_end_inclusive && ordering == Ordering::Equal)
        {
            let next_value =
                nu_data::value::compute_values(Operator::Plus, &self.curr, &self.negative_one);

            let mut next = match next_value {
                Ok(result) => result,
                Err((left_type, right_type)) => {
                    self.done = true;
                    return Some(
                        UntaggedValue::Error(ShellError::coerce_error(
                            left_type.spanned(self.tag.span),
                            right_type.spanned(self.tag.span),
                        ))
                        .into_untagged_value(),
                    );
                }
            };
            std::mem::swap(&mut self.curr, &mut next);

            Some(next.into_value(self.tag.clone()))
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
