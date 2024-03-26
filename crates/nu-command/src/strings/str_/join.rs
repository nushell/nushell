use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrJoin;

impl Command for StrJoin {
    fn name(&self) -> &str {
        "str join"
    }

    fn signature(&self) -> Signature {
        Signature::build("str join")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::String),
                (Type::String, Type::String),
            ])
            .optional(
                "separator",
                SyntaxShape::String,
                "Optional separator to use when creating string.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Concatenate multiple strings into a single string, with an optional separator between each."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["collect", "concatenate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<String> = call.opt(engine_state, stack, 0)?;

        let config = engine_state.get_config();

        // let output = input.collect_string(&separator.unwrap_or_default(), &config)?;
        // Hmm, not sure what we actually want.
        // `to_formatted_string` formats dates as human readable which feels funny.
        let mut strings: Vec<String> = vec![];

        for value in input {
            let str = match value {
                Value::Error { error, .. } => {
                    return Err(*error);
                }
                Value::Date { val, .. } => format!("{val:?}"),
                value => value.to_expanded_string("\n", config),
            };
            strings.push(str);
        }

        let output = if let Some(separator) = separator {
            strings.join(&separator)
        } else {
            strings.join("")
        };

        Ok(Value::string(output, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a string from input",
                example: "['nu', 'shell'] | str join",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Create a string from input with a separator",
                example: "['nu', 'shell'] | str join '-'",
                result: Some(Value::test_string("nu-shell")),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrJoin {})
    }
}
