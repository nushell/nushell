use dialoguer::{console::Term, Select};
use dialoguer::{FuzzySelect, MultiSelect};
use nu_ansi_term::Color;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
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
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::List(Box::new(Type::Any)), Type::Any),
            ])
            .optional("prompt", SyntaxShape::String, "the prompt to display")
            .switch(
                "multi",
                "Use multiple results, you can press a to toggle all options on/off",
                Some('m'),
            )
            .switch("fuzzy", "Use a fuzzy select.", Some('f'))
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

        let options: Vec<Options> = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. }
            | PipelineData::Value(Value::Record { .. }, ..) => {
                let mut lentable = Vec::<usize>::new();
                let rows = input.into_iter().collect::<Vec<_>>();
                rows.iter().for_each(|row| {
                    if let Ok(record) = row.as_record() {
                        let columns = record.len();
                        for (i, (col, val)) in record.iter().enumerate() {
                            if i == columns - 1 {
                                break;
                            }

                            if let Ok(val) = val.as_string() {
                                let len = nu_utils::strip_ansi_likely(&val).len()
                                    + nu_utils::strip_ansi_likely(col).len();
                                if let Some(max_len) = lentable.get(i) {
                                    lentable[i] = (*max_len).max(len);
                                } else {
                                    lentable.push(len);
                                }
                            }
                        }
                    }
                });

                rows.into_iter()
                    .map_while(move |x| {
                        if let Ok(val) = x.as_string() {
                            Some(Options {
                                name: val,
                                value: x,
                            })
                        } else if let Ok(record) = x.as_record() {
                            let mut options = Vec::new();
                            let columns = record.len();
                            for (i, (col, val)) in record.iter().enumerate() {
                                if let Ok(val) = val.as_string() {
                                    let len = nu_utils::strip_ansi_likely(&val).len()
                                        + nu_utils::strip_ansi_likely(col).len();
                                    options.push(format!(
                                        " {}{}{}: {}{}",
                                        Color::Cyan.prefix(),
                                        col,
                                        Color::Cyan.suffix(),
                                        &val,
                                        if i == columns - 1 {
                                            String::from("")
                                        } else {
                                            format!(
                                                "{} |",
                                                " ".repeat(
                                                    lentable
                                                        .get(i)
                                                        .cloned()
                                                        .unwrap_or_default()
                                                        .saturating_sub(len)
                                                )
                                            )
                                        }
                                    ));
                                }
                            }
                            Some(Options {
                                name: options.join(""),
                                value: x,
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }

            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected a list or table".to_string(),
                    span: head,
                })
            }
        };
        let prompt = prompt.unwrap_or_default();

        if options.is_empty() {
            return Err(ShellError::TypeMismatch {
                err_message: "expected a list or table, it can also be a problem with the an inner type of your list.".to_string(),
                span: head,
            });
        }

        if call.has_flag("multi") && call.has_flag("fuzzy") {
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

        let ans: InteractMode = if call.has_flag("multi") {
            let multi_select = MultiSelect::new(); //::with_theme(&theme);

            InteractMode::Multi(
                if !prompt.is_empty() {
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
        } else if call.has_flag("fuzzy") {
            let fuzzy_select = FuzzySelect::new(); //::with_theme(&theme);

            InteractMode::Single(
                if !prompt.is_empty() {
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
                if !prompt.is_empty() {
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
            InteractMode::Multi(res) => match res {
                Some(opts) => Value::list(
                    opts.iter().map(|s| options[*s].value.clone()).collect(),
                    head,
                ),
                None => Value::nothing(head),
            },
            InteractMode::Single(res) => match res {
                Some(opt) => options[opt].value.clone(),
                None => Value::nothing(head),
            },
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
