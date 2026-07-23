use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{engine::Closure, test_table, test_value};

#[derive(Clone)]
pub struct TakeUntil;

impl Command for TakeUntil {
    fn name(&self) -> &str {
        "take until"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .named(
                "include",
                SyntaxShape::Int,
                "Include extra items after the stream would otherwise have stopped. `0` is a no-op.",
                Some('i'),
            )
            .required(
                "predicate",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The predicate that element(s) must not match.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Take elements of the input until a predicate is true."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Take until the element is positive.",
                example: "[-1 -2 9 1] | take until {|x| $x > 0 }",
                result: Some(test_value!([-1, -2])),
            },
            Example {
                description: "Take until the element is positive using stored condition.",
                example: "let cond = {|x| $x > 0 }; [-1 -2 9 1] | take until $cond",
                result: Some(test_value!([-1, -2])),
            },
            Example {
                description: "Take until the field value is positive.",
                example: "[{a: -1} {a: -2} {a: 9} {a: 1}] | take until {|x| $x.a > 0 }",
                result: Some(test_value!([{a: (-1)}, {a: (-2)}])),
            },
            Example {
                description: "Take until the first item with an uppercase name including that item.",
                example: "[[name value]; [b, 2], [c, 3], [A, 1], [D, 4]] | take until -i 1 {|x| $x.name like '[A-Z]' }",
                result: Some(test_table![
                    ["name", "value"];
                    ["b", 2],
                    ["c", 3],
                    ["A", 1],
                ]),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let include: usize = call.get_flag(engine_state, stack, "include")?.unwrap_or(0);

        let metadata = input.take_metadata();

        let mut closure = ClosureEval::new(engine_state, stack, closure);
        let predicate = move |value: &Value| {
            closure
                .run_with_value(value.clone())
                .and_then(|data| data.into_value(head))
                .map(|cond| cond.is_false())
                .unwrap_or(false)
        };

        let it = input.into_iter_strict(head)?;

        Ok(match include {
            0 => it.take_while(predicate).into_pipeline_data_with_metadata(
                head,
                engine_state.signals().clone(),
                metadata,
            ),
            n => super::take_while_include::take_while_include_n(it, predicate, n)
                .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::TakeUntil;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(TakeUntil)
    }
}
