use dialoguer::MultiSelect;
use dialoguer::{console::Term, theme::ColorfulTheme, Select};
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
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Interactive list selection. Abort with esc or q"
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

        let options: Vec<String> = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..) => {
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
            }

            PipelineData::ListStream { .. } => input
                .into_iter()
                .map_while(move |x| {
                    let record = x.as_record().ok()?;

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
            PipelineData::Value(
                Value::Record {
                    cols,
                    vals,
                    span: _,
                },
                _,
            ) => {
                println!("cols: {:?}", cols);

                vals.iter()
                    .map(|x| x.as_string().unwrap_or_else(|_| "RECORD".to_string()))
                    .collect()
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
            InteractMode::Multi(
                MultiSelect::new()
                    .with_prompt(&prompt)
                    .items(&options)
                    .interact_on_opt(&Term::stderr())
                    .map_err(|_| {
                        ShellError::IOError("Oopsie, list input is a wip command...".to_owned())
                    })?,
            )
        } else {
            InteractMode::Single(
                Select::with_theme(&ColorfulTheme::default())
                    .items(&options)
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
                        vals: opts
                            .iter()
                            .map(|s| Value::String {
                                val: options[*s].clone(),
                                span: head,
                            })
                            .collect(),
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
                    Some(opt) => Value::String {
                        val: options[opt].clone(),
                        span: head,
                    },
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
