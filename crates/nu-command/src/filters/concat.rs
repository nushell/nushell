use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Concat;

impl Command for Concat {
    fn name(&self) -> &str {
        "concat"
    }

    fn description(&self) -> &str {
        "Read multiple streams sequentially and combine them into one uninterrupted stream."
    }

    fn extra_description(&self) -> &str {
        r#"If input is provided to `concat`, the input will be combined with the
output of the closures. This enables `concat` to be used at any position
within a pipeline.

A stream will begin to be consumed only after the preceding stream has ran out.
The output will consist of, in order:
  - the items from the pipeline input
  - the items from the 1st closure's return stream
  - the items from the 2nd closure's return stream
  ...
  - the items from the nth closure's return stream"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("concat")
            .input_output_types(vec![
                (Type::List(Type::Any.into()), Type::List(Type::Any.into())),
                (Type::Nothing, Type::List(Type::Any.into())),
            ])
            .rest(
                "closures",
                SyntaxShape::Closure(None),
                "These closures will run in order and their output streams will be.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Concatenate streams",
            example: "\
                seq 1 3 | each { $'number ($in)' }\n\
                | concat { seq char a c | each { $'char ($in)' } }\
            ",
            result: Some(Value::test_list(vec![
                Value::test_string("number 1"),
                Value::test_string("number 2"),
                Value::test_string("number 3"),
                Value::test_string("char a"),
                Value::test_string("char b"),
                Value::test_string("char c"),
            ])),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closures: Vec<Closure> = call.rest(engine_state, stack, 0)?;

        let metadata = input.metadata();

        let pipeline_datas = std::iter::once(Ok(input))
            .chain(closures.into_iter().map(|closure| {
                ClosureEvalOnce::new(engine_state, stack, closure)
                    .run_with_input(PipelineData::empty())
            }))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(pipeline_datas
            .into_iter()
            .flatten()
            .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Concat {})
    }
}
