
use super::utils::chain_error_with_input;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct PartitionBy;

impl Command for PartitionBy {
    fn name(&self) -> &str {
        "partition-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("partition-by")
            .input_output_types(
                vec![
                (Type::List(Box::new(Type::Any)), 
                 Type::List(Box::new(Type::Any)))
                ])
            .required(
                "clsoure",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The closure to partition on",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Splits a list into partitions, and returns a list containing those partitions."
    }

    fn extra_description(&self) -> &str {
        r#"TODO"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        partition_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

struct Partition<I, T, F, K> {
    iterator: I,
    last_value: Option<(T, K)>,
    closure: F,
    done: bool
}

impl<I,T,F,K> Iterator for Partition<I, T, F, K> where 
    I : Iterator<Item=T>,
    F : FnMut(&T) -> K,
    K : PartialEq {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {

        if self.done {
            return None;
        }
        
        let (head, head_key) = match self.last_value.take() {
            None => {
                let head = self.iterator.next()?;

                let key = (self.closure)(&head);

                (head, key)
            },

            Some((value, key)) => { (value,key) }
        };

        let mut result = vec![head];

        loop {
            match self.iterator.next() {
                None =>  {
                    self.done = true;
                    return Some(result);
                }
                Some(value) => {
                    let value_key = (self.closure)(&value);

                    if value_key == head_key {
                        result.push(value);
                    }
                    else {
                        self.last_value = Some((value, value_key));
                        return Some(result);
                    }
                }
            }
        }
    }
}

fn partition_iter_by<I,T,F,K>(iterator: I, closure: F) -> Partition<I,T,F,K> 
    where I : Iterator<Item=T>,
    F : FnMut(&T) -> K, {

    return Partition {
        closure,
        iterator,
        last_value: None,
        done: false
    }
}

pub fn partition_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let closure: Closure = call.req(engine_state, stack, 0)?;

    let metadata = input.metadata();

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream(..) => {

                let mut closure = ClosureEval::new(engine_state, stack, closure);

                let result = 
                    partition_iter_by(input.into_iter(), 
                        move |value| { 
                            match closure.run_with_value(value.clone()) {
                                Ok(data) => data.into_value(head).unwrap_or_else(|_| {
                                    todo!("handle this")
                                }),

                                Err(_) =>  todo!("also handle this") 
                            }
                        })
                    .map(move |it| Value::list(it, head));

                Ok(result.into_pipeline_data(head, engine_state.signals().clone()))
                    


            }
        PipelineData::ByteStream(stream, ..) => {
            if let Some(chunks) = stream.chunks() {
                let mut closure = ClosureEval::new(engine_state, stack, closure);
                Ok(chunks
                    .map_while(move |value| {
                        let value = match value {
                            Ok(value) => value,
                            Err(err) => return Some(Value::error(err, head)),
                        };

                        let span = value.span();
                        let is_error = value.is_error();
                        match closure
                            .run_with_value(value)
                            .and_then(|data| data.into_value(head))
                            {
                                Ok(value) => Some(value),
                                Err(error) => {
                                    let error = chain_error_with_input(error, is_error, span);
                                    Some(Value::error(error, span))
                                }
                            }
                    })
                    .into_pipeline_data(head, engine_state.signals().clone()))
            } else {
                Ok(PipelineData::Empty)
            }
        }

        PipelineData::Value(_, ..) => {
            todo!("raise nushell error");
        }
    }
    .map(|data| data.set_metadata(metadata))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PartitionBy {})
    }
}
