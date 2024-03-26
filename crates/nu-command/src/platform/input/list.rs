use dialoguer::{console::Term, Select};
use dialoguer::{FuzzySelect, MultiSelect};
use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};
use std::fmt::{Display, Formatter};

enum InteractMode {
    Single(Option<usize>),
    Multi(Option<Vec<usize>>),
}

#[derive(Clone)]
struct Options {
    name: String,
    value: Value,
}

impl Display for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone)]
pub struct InputList;

const INTERACT_ERROR: &str = "Interact error, could not process options";

impl Command for InputList {
    fn name(&self) -> &str {
        "input list"
    }

    fn signature(&self) -> Signature {
        Signature::build("input list")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::Range, Type::Int),
            ])
            .optional("prompt", SyntaxShape::String, "The prompt to display.")
            .switch(
                "multi",
                "Use multiple results, you can press a to toggle all options on/off",
                Some('m'),
            )
            .switch("fuzzy", "Use a fuzzy select.", Some('f'))
            .switch("index", "Returns list indexes.", Some('i'))
            .named(
                "display",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "Field to use as display value, or closure to compute it",
                Some('d'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Interactive list selection."
    }

    fn extra_usage(&self) -> &str {
        "Abort with esc or q."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "ask", "menu"]
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
        let multi = call.has_flag(engine_state, stack, "multi")?;
        let fuzzy = call.has_flag(engine_state, stack, "fuzzy")?;
        let index = call.has_flag(engine_state, stack, "index")?;
        let display_arg: Option<Value> = call.get_flag(engine_state, stack, "display")?;

        let options: Vec<Options> = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => {
                let labels: Result<Vec<Options>, ShellError> = input
                    .into_iter()
                    .map(move |item: Value| {
                        if let Some(display_arg) = &display_arg {
                            match display_arg {
                                // We match the closure that creates the display strings depending on whether the user haas given us a cell path/closure/nothing to use.
                                Value::CellPath { val, .. } => Ok(Options {
                                    name: item
                                        .clone()
                                        .follow_cell_path(&val.members, false)?
                                        .to_expanded_string(", ", engine_state.get_config()),
                                    value: item,
                                }),
                                Value::Closure { val, .. } => {
                                    // See insert.rs:insert_value_by_closure
                                    let span = display_arg.span();
                                    let closure = val;
                                    let block = engine_state.get_block(closure.block_id);
                                    let mut stack_clone = stack.captures_to_stack(closure.captures.clone()).clone();  // Create a stack for our closure, containing the vars it captures.

                                    // Closure argument
                                    if let Some(var) = block.signature.get_positional(0) {  // If the closure takes a positional argument (ie.: {|arg| ... })
                                        if let Some(var_id) = &var.var_id {
                                            stack_clone.add_var(*var_id, item.clone());        // Add the item into the place of that argument.
                                        }
                                    }
                                    
                                    // Closure pipeline input
                                    let closure_input = item.clone().into_pipeline_data();
                                    
                                    let output = eval_block(
                                        engine_state,
                                        &mut stack_clone,
                                        block,
                                        closure_input,
                                        call.redirect_stdout,
                                        call.redirect_stderr,
                                    )?;

                                    Ok(Options {
                                        name: output
                                            .into_value(span)
                                            .to_expanded_string(", ", engine_state.get_config()),
                                        value: item,
                                    })
                                }
                                _ => Err(ShellError::TypeMismatch {
                                    err_message: "--display expects either a cell path or a closure"
                                        .to_string(),
                                    span: head,
                                }),
                            }
                        }
                        else {
                            Ok(Options {
                                name: item.to_expanded_string(", ", engine_state.get_config()),
                                value: item,
                            })
                        }
                    })
                    .collect();

                labels?
            }

            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected a list, a table, or a range".to_string(),
                    span: head,
                })
            }
        };

        if options.is_empty() {
            return Err(ShellError::TypeMismatch {
                err_message: "expected a list or table, it can also be a problem with the an inner type of your list.".to_string(),
                span: head,
            });
        }

        if multi && fuzzy {
            return Err(ShellError::TypeMismatch {
                err_message: "Fuzzy search is not supported for multi select".to_string(),
                span: head,
            });
        }

        // could potentially be used to map the use theme colors at some point
        // let theme = dialoguer::theme::ColorfulTheme {
        //     active_item_style: Style::new().fg(Color::Cyan).bold(),
        //     ..Default::default()
        // };

        let ans: InteractMode = if multi {
            let multi_select = MultiSelect::new(); //::with_theme(&theme);

            InteractMode::Multi(
                if let Some(prompt) = prompt {
                    multi_select.with_prompt(&prompt)
                } else {
                    multi_select
                }
                .items(&options)
                .report(false)
                .interact_on_opt(&Term::stderr())
                .map_err(|err| ShellError::IOError {
                    msg: format!("{}: {}", INTERACT_ERROR, err),
                })?,
            )
        } else if fuzzy {
            let fuzzy_select = FuzzySelect::new(); //::with_theme(&theme);

            InteractMode::Single(
                if let Some(prompt) = prompt {
                    fuzzy_select.with_prompt(&prompt)
                } else {
                    fuzzy_select
                }
                .items(&options)
                .default(0)
                .report(false)
                .interact_on_opt(&Term::stderr())
                .map_err(|err| ShellError::IOError {
                    msg: format!("{}: {}", INTERACT_ERROR, err),
                })?,
            )
        } else {
            let select = Select::new(); //::with_theme(&theme);
            InteractMode::Single(
                if let Some(prompt) = prompt {
                    select.with_prompt(&prompt)
                } else {
                    select
                }
                .items(&options)
                .default(0)
                .report(false)
                .interact_on_opt(&Term::stderr())
                .map_err(|err| ShellError::IOError {
                    msg: format!("{}: {}", INTERACT_ERROR, err),
                })?,
            )
        };

        Ok(match ans {
            InteractMode::Multi(res) => {
                if index {
                    match res {
                        Some(opts) => Value::list(
                            opts.into_iter()
                                .map(|s| Value::int(s as i64, head))
                                .collect(),
                            head,
                        ),
                        None => Value::nothing(head),
                    }
                } else {
                    match res {
                        Some(opts) => Value::list(
                            opts.iter().map(|s| options[*s].value.clone()).collect(),
                            head,
                        ),
                        None => Value::nothing(head),
                    }
                }
            }
            InteractMode::Single(res) => {
                if index {
                    match res {
                        Some(opt) => Value::int(opt as i64, head),
                        None => Value::nothing(head),
                    }
                } else {
                    match res {
                        Some(opt) => options[opt].value.clone(),
                        None => Value::nothing(head),
                    }
                }
            }
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return a single value from a list",
                example: r#"[1 2 3 4 5] | input list 'Rate it'"#,
                result: None,
            },
            Example {
                description: "Return multiple values from a list",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --multi 'Add fruits to the basket'"#,
                result: None,
            },
            Example {
                description: "Return a single record from a table with fuzzy search",
                example: r#"ls | input list --fuzzy 'Select the target'"#,
                result: None,
            },
            Example {
                description: "Choose an item from a range",
                example: r#"1..10 | input list"#,
                result: None,
            },
            Example {
                description: "Return the index of a selected item",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --index"#,
                result: None,
            },
            Example {
                description: "Choose an item from a table using a column as display value",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d $.name"#,
                result: None,
            },
            Example {
                description: "Choose an item using a closure to compute display value",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d {|| get name}"#,
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(InputList {})
    }
}
