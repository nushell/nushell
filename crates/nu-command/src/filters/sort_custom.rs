use itertools::Itertools;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

use std::cmp::Ordering;

#[derive(Clone)]
pub struct SortCustom;

impl Command for SortCustom {
    fn name(&self) -> &str {
        "sort custom"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort custom")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::record(), Type::record()),
            ])
            .switch("reverse", "Sort in reverse order", Some('r'))
            .category(Category::Filters)
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "The closure to compare elements with.",
            )
    }

    fn usage(&self) -> &str {
        "Sort using a comparator closure."
    }

    // TODO
    // fn examples(&self) -> Vec<Example> {
    // }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let metadata = input.metadata();

        let closure: Closure = call.req(engine_state, stack, 0)?;
        let mut closure_eval = ClosureEval::new(engine_state, stack, closure);

        let mut closure_err: Option<ShellError> = None;

        let comparator = |a: &Value, b: &Value| {
            match closure_eval
                .add_arg(a.clone())
                .add_arg(b.clone())
                .run_with_input(PipelineData::Empty)
                .and_then(|data| data.into_value(head))
            {
                Ok(cond) => {
                    if cond.is_true() ^ reverse {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                }
                Err(err) => {
                    // show user only the first error that occurred
                    if closure_err.is_none() {
                        closure_err = Some(err.clone());
                    }
                    // TODO: handle error at time it was caused?
                    Ordering::Equal
                }
            }
        };

        let out = input
            .into_iter()
            .sorted_by(comparator)
            .into_pipeline_data_with_metadata(head, engine_state.ctrlc.clone(), metadata);

        if let Some(err) = closure_err {
            Err(err)
        } else {
            Ok(out)
        }
    }
}

// #[cfg(test)]
// mod test {

//     use nu_protocol::engine::CommandType;

//     use super::*;

//     #[test]
//     fn test_examples() {
//         use crate::test_examples;

//         test_examples(SortCustom {})
//     }

//     #[test]
//     fn test_command_type() {
//         assert!(matches!(SortCustom.command_type(), CommandType::Builtin));
//     }
// }
