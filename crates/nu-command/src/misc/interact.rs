use inquire::Select;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Interact;

impl Command for Interact {
    fn name(&self) -> &str {
        "interact"
    }

    fn signature(&self) -> Signature {
        Signature::build("interact")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::String),
                (Type::String, Type::String),
            ])
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        // Lots of great examples here:
        // https://github.com/mikaelmello/inquire

        // let options = vec![
        //     "Banana",
        //     "Apple",
        //     "Strawberry",
        //     "Grapes",
        //     "Lemon",
        //     "Tangerine",
        //     "Watermelon",
        //     "Orange",
        //     "Pear",
        //     "Avocado",
        //     "Pineapple",
        // ];

        let options = match input {
            PipelineData::Value(Value::List { vals, .. }, _) => {
                let mut options = Vec::new();
                for val in vals {
                    match val {
                        // Value::String { val, .. } => options.push(val),
                        // _ => return Err(ShellError::type_error("string", val.type_name())),
                        _ => options.push(val.as_string()?),
                    }
                }
                options
            }
            PipelineData::Value(Value::String { val, .. }, _) => vec![val],
            // _ => return Err(ShellError::type_error("string", input.type_name())),
            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected string or list".to_string(),
                    span: head,
                })
            }
        };

        let ans = Select::new("What's your favorite fruit?", options).prompt();

        let answer = match ans {
            Ok(choice) => format!("{choice}! That's mine too!"),
            Err(_) => "There was an error, please try again".to_string(),
        };

        Ok(Value::String {
            val: answer,
            span: head,
        }
        .into_pipeline_data())
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
