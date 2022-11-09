use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Group;

impl Command for Group {
    fn name(&self) -> &str {
        "group"
    }

    fn signature(&self) -> Signature {
        Signature::build("group")
            // TODO: It accepts Table also, but currently there is no Table
            // example. Perhaps Table should be a subtype of List, in which case
            // the current signature would suffice even when a Table example
            // exists.
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::List(Box::new(Type::Any)))),
            )])
            .required("group_size", SyntaxShape::Int, "the size of each group")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Groups input into groups of `group_size`."
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 3,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 4,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
        ];

        vec![Example {
            example: "echo [1 2 3 4] | group 2",
            description: "Group the a list by pairs",
            result: Some(Value::List {
                vals: stream_test_1,
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let group_size: Spanned<usize> = call.req(engine_state, stack, 0)?;
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachGroupIterator {
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
            span: call.head,
        };

        Ok(each_group_iterator
            .into_pipeline_data(ctrlc)
            .set_metadata(metadata))
    }
}

struct EachGroupIterator {
    group_size: usize,
    input: Box<dyn Iterator<Item = Value> + Send>,
    span: Span,
}

impl Iterator for EachGroupIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut group = vec![];
        let mut current_count = 0;

        loop {
            let item = self.input.next();

            match item {
                Some(v) => {
                    group.push(v);

                    current_count += 1;
                    if current_count >= self.group_size {
                        break;
                    }
                }
                None => break,
            }
        }

        if group.is_empty() {
            return None;
        }

        Some(Value::List {
            vals: group,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Group {})
    }
}
