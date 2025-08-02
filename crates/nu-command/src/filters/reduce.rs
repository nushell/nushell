use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Reduce;

impl Command for Reduce {
    fn name(&self) -> &str {
        "reduce"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::table(), Type::Any),
                (Type::Range, Type::Any),
            ])
            .named(
                "fold",
                SyntaxShape::Any,
                "reduce with initial value",
                Some('f'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "Reducing function.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Aggregate a list (starting from the left) to a single value using an accumulator closure."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["map", "fold", "foldl"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[ 1 2 3 4 ] | reduce {|it, acc| $it + $acc }",
                description: "Sum values of a list (same as 'math sum')",
                result: Some(Value::test_int(10)),
            },
            Example {
                example: "[ 1 2 3 4 ] | reduce {|it, acc| $acc - $it }",
                description: r#"`reduce` accumulates value from left to right, equivalent to (((1 - 2) - 3) - 4)."#,
                result: Some(Value::test_int(-8)),
            },
            Example {
                example: "[ 8 7 6 ] | enumerate | reduce --fold 0 {|it, acc| $acc + $it.item + $it.index }",
                description: "Sum values of a list, plus their indexes",
                result: Some(Value::test_int(24)),
            },
            Example {
                example: "[ 1 2 3 4 ] | reduce --fold 10 {|it, acc| $acc + $it }",
                description: "Sum values with a starting value (fold)",
                result: Some(Value::test_int(20)),
            },
            Example {
                example: r#"[[foo baz] [baz quux]] | reduce --fold "foobar" {|it, acc| $acc | str replace $it.0 $it.1}"#,
                description: "Iteratively perform string replace (from left to right): 'foobar' -> 'bazbar' -> 'quuxbar'",
                result: Some(Value::test_string("quuxbar")),
            },
            Example {
                example: r#"[ i o t ] | reduce --fold "Arthur, King of the Britons" {|it, acc| $acc | str replace --all $it "X" }"#,
                description: "Replace selected characters in a string with 'X'",
                result: Some(Value::test_string("ArXhur, KXng Xf Xhe BrXXXns")),
            },
            Example {
                example: r#"['foo.gz', 'bar.gz', 'baz.gz'] | enumerate | reduce --fold '' {|str all| $"($all)(if $str.index != 0 {'; '})($str.index + 1)-($str.item)" }"#,
                description: "Add ascending numbers to each of the filenames, and join with semicolons.",
                result: Some(Value::test_string("1-foo.gz; 2-bar.gz; 3-baz.gz")),
            },
            Example {
                example: r#"let s = "Str"; 0..2 | reduce --fold '' {|it, acc| $acc + $s}"#,
                description: "Concatenate a string with itself, using a range to determine the number of times.",
                result: Some(Value::test_string("StrStrStr")),
            },
            Example {
                example: r#"[{a: 1} {b: 2} {c: 3}] | reduce {|it| merge $it}"#,
                description: "Merge multiple records together, making use of the fact that the accumulated value is also supplied as pipeline input to the closure.",
                result: Some(Value::test_record(record!(
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                    "c" => Value::test_int(3),
                ))),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let fold: Option<Value> = call.get_flag(engine_state, stack, "fold")?;
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let mut iter = input.into_iter();

        let mut acc = fold
            .or_else(|| iter.next())
            .ok_or_else(|| ShellError::GenericError {
                error: "Expected input".into(),
                msg: "needs input".into(),
                span: Some(head),
                help: None,
                inner: vec![],
            })?;

        let mut closure = ClosureEval::new(engine_state, stack, closure);

        for value in iter {
            engine_state.signals().check(&head)?;
            acc = closure
                .add_arg(value)
                .add_arg(acc.clone())
                .run_with_input(PipelineData::value(acc, None))?
                .into_value(head)?;
        }

        Ok(acc.with_span(head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::{Merge, test_examples_with_commands};

        test_examples_with_commands(Reduce {}, &[&Merge])
    }
}
