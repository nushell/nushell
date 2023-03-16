// use inquire::list_option::ListOption;
use inquire::{MultiSelect, Select};
use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, /*IntoInterruptiblePipelineData,*/ IntoPipelineData, PipelineData,
    ShellError, Signature, SyntaxShape, Type, Value,
};

// TODO:
// - implement more interact modes
// - add support for validation / formatting closures
// - add support for customizing the prompts

enum InteractMode {
    Single(String),
    Multi(Vec<String>),
}

#[derive(Clone)]
pub struct Interact;

impl Command for Interact {
    fn name(&self) -> &str {
        "interact"
    }

    fn signature(&self) -> Signature {
        Signature::build("interact")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::List(Box::new(Type::Any))),
                // (Type::List(Box::new(Type::Any)), Type::Any),
                // (Type::String, Type::String),
            ])
            .optional("prompt", SyntaxShape::String, "the prompt to display")
            .switch("multi", "Use multiple results", Some('m'))
            // record index uszie value string
            .optional(
                "validator",
                SyntaxShape::Closure(Some(vec![
                    SyntaxShape::List(Box::new(SyntaxShape::Record)),
                    SyntaxShape::Int,
                ])),
                "validator for the selection (not implemented yet)",
            )
            .allow_variants_without_examples(true)
            .category(Category::Misc)
    }

    fn usage(&self) -> &str {
        "Show interactive menus."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "ask", "input", "menu"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;

        // let capture_block: Closure = call.req(engine_state, stack, 0)?;

        // let block = engine_state.get_block(capture_block.block_id).clone();
        // let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        // let mut stack = stack.captures_to_stack(&capture_block.captures);

        // let redirect_stdout = call.redirect_stdout;
        // let redirect_stderr = call.redirect_stderr;

        // Lots of great examples here:
        // https://github.com/mikaelmello/inquire

        // let mut data: PipelineData = PipelineData::Empty;
        // let validator = |a: &[ListOption<&&str>]| {
        //     let data: PipelineData = a
        //         .iter()
        //         // map the record index uszie value string
        //         .map(|s| Value::Record {
        //             cols: vec!["index".to_string(), "value".to_string()],
        //             vals: vec![
        //                 Value::Int {
        //                     val: s.index.to_owned() as i64,
        //                     span: head,
        //                 },
        //                 Value::String {
        //                     val: s.value.to_owned().to_string(),
        //                     span: head,
        //                 },
        //             ],
        //             span: head,
        //         })
        //         .into_pipeline_data(None);
        // .collect();
        //if let Some(var_id) = var_id {
        //     stack.add_var(var_id, data);
        // }
        // eval_block(
        //     &engine_state,
        //     &mut stack,
        //     &block,
        //     data,
        //     redirect_stdout,
        //     redirect_stderr,
        // );
        // };

        let options = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..) => {
                //| PipelineData::ListStream { .. } => {
                // let mut options = Vec::new();
                input
                    .into_iter()
                    .map_while(move |x| {
                        // check if x is a string or a record

                        if let Ok(val) = x.as_string() {
                            Some(val)
                        } else if let Ok(record) = x.as_record() {
                            let mut options = Vec::new();
                            for (col, val) in record.0.iter().zip(record.1.iter()) {
                                if let Ok(val) = val.as_string() {
                                    options.push(format!(" {}: {} |", &col, &val));
                                }
                            }
                            Some(options.join(""))
                        } else {
                            None
                        }
                    })
                    .collect()
                // for val in vals {
                //     match val {
                //         // Value::String { val, .. } => options.push(val),
                //         // _ => return Err(ShellError::type_error("string", val.type_name())),
                //         _ => options.push(val.as_string()?),
                //     }
                // }
                // options
            }
            // PipelineData::Value(Value::String { val, .. }, _) => vec![val],
            // _ => return Err(ShellError::type_error("string", input.type_name())),
            PipelineData::ListStream { .. } => input
                .into_iter()
                .map_while(move |x| {
                    let record = x.as_record().unwrap();

                    record
                        .0
                        .iter()
                        .zip(record.1.iter())
                        .map(|(col, val)| {
                            println!("col: {:?}", col);
                            println!("val: {:?}", val);
                            if let Ok(val) = val.as_string() {
                                Some(format!(" {}: {} |", &col, &val))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .collect(),
            PipelineData::Value(Value::Record { cols, vals, span }, _) => {
                // if let Some(var) = block.signature.get_positional(0) {
                //     if let Some(var_id) = &var.var_id {
                //         stack.add_var(*var_id, x.clone());
                //     }
                // }

                println!("cols: {:?}", cols);

                vals.iter()
                    .map(|x| x.as_string().unwrap_or_else(|_| "RECORD".to_string()))
                    .collect()

                // eval_block_with_early_return(
                //     &engine_state,
                //     &mut stack,
                //     &block,
                //     x.into_pipeline_data(),
                //     redirect_stdout,
                //     redirect_stderr,
                // )
            }
            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected string or list".to_string(),
                    span: head,
                })
            }
        };
        let prompt = prompt.unwrap_or_default();

        let ans: InteractMode = if call.has_flag("multi") {
            InteractMode::Multi(MultiSelect::new(&prompt, options).prompt().map_err(|_| {
                ShellError::IOError("Oopsie, interact is a wip command...".to_owned())
            })?)
        } else {
            InteractMode::Single(Select::new(&prompt, options).prompt().map_err(|_| {
                ShellError::IOError("Oopsie, interact is a wip command...".to_owned())
            })?)
        };

        match ans {
            InteractMode::Multi(res) => Ok(Value::List {
                vals: res
                    .iter()
                    .map(|s| Value::String {
                        val: s.clone(),
                        span: head,
                    })
                    .collect(),
                span: head,
            }
            .into_pipeline_data()),
            InteractMode::Single(res) => Ok(Value::String {
                val: res,
                span: head,
            }
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print an interactive menu and wait for a response.",
            example: r#"[1 2 3] | interact"#,
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Interact {})
    }
}
