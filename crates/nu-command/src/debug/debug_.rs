use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Debug;

impl Command for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn description(&self) -> &str {
        "Debug print the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("debug")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Any, Type::String),
            ])
            .category(Category::Debug)
            .switch("raw", "Prints the raw value representation", Some('r'))
            .switch(
                "raw-value",
                "Prints the raw value representation but not the nushell value part",
                Some('v'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let config = stack.get_config(engine_state);
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let raw_value = call.has_flag(engine_state, stack, "raw-value")?;

        // Should PipelineData::empty() result in an error here?

        input.map(
            move |x| {
                if raw {
                    Value::string(x.to_debug_string(), head)
                } else if raw_value {
                    match x.coerce_into_string_all() {
                        Ok(s) => Value::string(format!("{s:#?}"), head),
                        Err(e) => Value::error(e, head),
                    }
                } else {
                    Value::string(x.to_expanded_string(", ", &config), head)
                }
            },
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Debug print a string",
                example: "'hello' | debug",
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Debug print a list",
                example: "['hello'] | debug",
                result: Some(Value::list(
                    vec![Value::test_string("hello")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Debug print a table",
                example: "[[version patch]; ['0.1.0' false] ['0.1.1' true] ['0.2.0' false]] | debug",
                result: Some(Value::list(
                    vec![
                        Value::test_string("{version: 0.1.0, patch: false}"),
                        Value::test_string("{version: 0.1.1, patch: true}"),
                        Value::test_string("{version: 0.2.0, patch: false}"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Debug print an ansi escape encoded string and get the raw value",
                example: "$'(ansi red)nushell(ansi reset)' | debug -v",
                result: Some(Value::test_string("\"\\u{1b}[31mnushell\\u{1b}[0m\"")),
            },
        ]
    }
}

// This is just a local Value Extension trait to avoid having to
// put another *_to_string() converter in nu_protocol
trait ValueExt {
    fn coerce_into_string_all(&self) -> Result<String, ShellError>;
    fn cant_convert_to<T>(&self, typ: &str) -> Result<T, ShellError>;
}

impl ValueExt for Value {
    fn cant_convert_to<T>(&self, typ: &str) -> Result<T, ShellError> {
        Err(ShellError::CantConvert {
            to_type: typ.into(),
            from_type: self.get_type().to_string(),
            span: self.span(),
            help: None,
        })
    }

    fn coerce_into_string_all(&self) -> Result<String, ShellError> {
        let span = self.span();
        match self {
            Value::Bool { val, .. } => Ok(val.to_string()),
            Value::Int { val, .. } => Ok(val.to_string()),
            Value::Float { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val.to_string()),
            Value::Glob { val, .. } => Ok(val.to_string()),
            Value::Filesize { val, .. } => Ok(val.get().to_string()),
            Value::Duration { val, .. } => Ok(val.to_string()),
            Value::Date { val, .. } => Ok(val.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
            Value::Range { val, .. } => Ok(val.to_string()),
            Value::Record { val, .. } => Ok(format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| match y.coerce_into_string_all() {
                        Ok(value) => format!("{x}: {value}"),
                        Err(err) => format!("Error: {err}"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Value::List { vals, .. } => Ok(format!(
                "[{}]",
                vals.iter()
                    .map(|x| match x.coerce_into_string_all() {
                        Ok(value) => value,
                        Err(err) => format!("Error: {err}"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Value::Binary { val, .. } => match String::from_utf8(val.to_vec()) {
                Ok(s) => Ok(s),
                Err(err) => Value::binary(err.into_bytes(), span).cant_convert_to("string"),
            },
            Value::CellPath { val, .. } => Ok(val.to_string()),
            Value::Nothing { .. } => Ok("nothing".to_string()),
            val => val.cant_convert_to("string"),
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Debug;
        use crate::test_examples;
        test_examples(Debug {})
    }
}
