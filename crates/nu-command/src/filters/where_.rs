use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn usage(&self) -> &str {
        "Filter values based on a row condition."
    }

    fn extra_usage(&self) -> &str {
        r#"This command works similar to 'filter' but allows extra shorthands for working with
tables, known as "row conditions". On the other hand, reading the condition from a variable is
not supported."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("where")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .required(
                "row_condition",
                SyntaxShape::RowCondition,
                "Filter condition",
            )
            // TODO: Remove this flag after 0.73.0 release
            .named(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "use with a closure instead (deprecated: use 'filter' command instead)",
                Some('b'),
            )
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "find", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        if let Some(closure) = call.get_flag::<Spanned<Closure>>(engine_state, stack, "closure")? {
            return Err(ShellError::DeprecatedParameter(
                "-b, --closure".to_string(),
                "filter command".to_string(),
                closure.span,
            ));
        }

        let closure: Closure = call.req(engine_state, stack, 0)?;

        let span = call.head;

        let metadata = input.metadata();
        let mut stack = stack.captures_to_stack(&closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();

        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;
        Ok(input
            .into_iter()
            .enumerate()
            .filter_map(move |(idx, value)| {
                stack.with_env(&orig_env_vars, &orig_env_hidden);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, value.clone());
                    }
                }
                // Optional index argument
                if let Some(var) = block.signature.get_positional(1) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(
                            *var_id,
                            Value::Int {
                                val: idx as i64,
                                span,
                            },
                        );
                    }
                }
                let result = eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    // clone() is used here because x is given to Ok() below.
                    value.clone().into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                );

                match result {
                    Ok(result) => {
                        let result = result.into_value(span);
                        if result.is_true() {
                            Some(value)
                        } else {
                            None
                        }
                    }
                    Err(err) => Some(Value::Error { error: err }),
                }
            })
            .into_pipeline_data(ctrlc))
        .map(|x| x.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | where a > 1",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["a".to_string()],
                        vals: vec![Value::test_int(2)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | where {|x| $x > 1}",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "List all files in the current directory with sizes greater than 2kb",
                example: "ls | where size > 2kb",
                result: None,
            },
            Example {
                description: "List only the files in the current directory",
                example: "ls | where type == file",
                result: None,
            },
            Example {
                description: "List all files with names that contain \"Car\"",
                example: "ls | where name =~ \"Car\"",
                result: None,
            },
            Example {
                description: "List all files that were modified in the last two weeks",
                example: "ls | where modified >= (date now) - 2wk",
                result: None,
            },
            // TODO: This should work but does not. (Note that `Let` must be present in the working_set in `example_test.rs`).
            // See https://github.com/nushell/nushell/issues/7034
            // Example {
            //     description: "List all numbers above 3, using an existing closure condition",
            //     example: "let a = {$in > 3}; [1, 2, 5, 6] | where -b $a",
            //     result: Some(Value::List {
            //         vals: vec![
            //             Value::Int {
            //                 val: 5,
            //                 span: Span::test_data(),
            //             },
            //             Value::Int {
            //                 val: 6,
            //                 span: Span::test_data(),
            //             },
            //         ],
            //         span: Span::test_data(),
            //     }),
            // },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Where {})
    }
}
