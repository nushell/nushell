use nu_ansi_term::Color;
use std::fmt::{Display, Formatter};

use dialoguer::{console::Term, theme::ColorfulTheme, Select};
use dialoguer::{FuzzySelect, MultiSelect};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};

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

impl Command for InputList {
    fn name(&self) -> &str {
        "input list"
    }

    fn signature(&self) -> Signature {
        Signature::build("input list")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
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
            | PipelineData::Value(Value::Record { .. }, ..) => input
                .into_iter()
                .map_while(move |x| {
                    if let Ok(val) = x.as_string() {
                        Some(Options {
                            name: val,
                            value: x,
                        })
                    } else if let Ok(record) = x.as_record() {
                        let mut options = Vec::new();
                        for (col, val) in record.0.iter().zip(record.1.iter()) {
                            if let Ok(val) = val.as_string() {
                                options.push(format!(
                                    " {}{}{}: {} |\t",
                                    Color::Cyan.prefix(),
                                    col,
                                    Color::Cyan.suffix(),
                                    &val
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
                .collect(),

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

        let ans: InteractMode = if call.has_flag("multi") {
            if call.has_flag("fuzzy") {
                return Err(ShellError::TypeMismatch {
                    err_message: "Fuzzy search is not supported for multi select".to_string(),
                    span: head,
                });
            } else {
                InteractMode::Multi(
                    MultiSelect::new()
                        .with_prompt(&prompt)
                        .report(false)
                        .items(&options)
                        .interact_on_opt(&Term::stderr())
                        .map_err(|_| {
                            ShellError::IOError("Oopsie, list input is a wip command...".to_owned())
                        })?,
                )
            }
        } else if call.has_flag("fuzzy") {
            InteractMode::Single(
                FuzzySelect::with_theme(&ColorfulTheme::default())
                    .items(&options)
                    .report(false)
                    .with_prompt(&prompt)
                    .interact_on_opt(&Term::stderr())
                    .map_err(|_| {
                        ShellError::IOError("Oopsie, list input is a wip command...".to_owned())
                    })?,
            )
        } else {
            InteractMode::Single(
                Select::with_theme(&ColorfulTheme::default())
                    .items(&options)
                    .report(false)
                    .with_prompt(&prompt)
                    .interact_on_opt(&Term::stderr())
                    .map_err(|_| {
                        ShellError::IOError("Oopsie, list input is a wip command...".to_owned())
                    })?,
            )
        };

        match ans {
            InteractMode::Multi(res) => Ok({
                match res {
                    Some(opts) => Value::List {
                        vals: opts.iter().map(|s| options[*s].value.clone()).collect(),
                        span: head,
                    },
                    None => Value::List {
                        vals: vec![],
                        span: head,
                    },
                }
            }
            .into_pipeline_data()),
            InteractMode::Single(res) => Ok({
                match res {
                    Some(opt) => options[opt].value.clone(),

                    None => Value::String {
                        val: "".to_string(),
                        span: head,
                    },
                }
            }
            .into_pipeline_data()),
        }
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
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list -m 'Add fruits to the basket'"#,
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
