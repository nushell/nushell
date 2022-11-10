use chrono_humanize::HumanTime;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    format_duration, format_filesize_from_conf, Category, Config, Example, IntoPipelineData,
    PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct ToText;

impl Command for ToText {
    fn name(&self) -> &str {
        "to text"
    }

    fn signature(&self) -> Signature {
        Signature::build("to text")
            .input_output_types(vec![(Type::Any, Type::String)])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts data into simple text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let span = call.head;
        let config = engine_state.get_config();

        let line_ending = if cfg!(target_os = "windows") {
            "\r\n"
        } else {
            "\n"
        };

        let collected_input = local_into_string(input.into_value(span), line_ending, config);

        Ok(Value::String {
            val: collected_input,
            span,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs data as simple text",
                example: "1 | to text",
                result: Some(Value::test_string("1")),
            },
            Example {
                description: "Outputs external data as simple text",
                example: "git help -a | lines | find -r '^ ' |  to text",
                result: None,
            },
            Example {
                description: "Outputs records as simple text",
                example: "ls |  to text",
                result: None,
            },
        ]
    }
}

fn local_into_string(value: Value, separator: &str, config: &Config) -> String {
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Filesize { val, .. } => format_filesize_from_conf(val, config),
        Value::Duration { val, .. } => format_duration(val),
        Value::Date { val, .. } => {
            format!("{} ({})", val.to_rfc2822(), HumanTime::from(val))
        }
        Value::Range { val, .. } => {
            format!(
                "{}..{}",
                local_into_string(val.from, ", ", config),
                local_into_string(val.to, ", ", config)
            )
        }
        Value::String { val, .. } => val,
        Value::List { vals: val, .. } => val
            .iter()
            .map(|x| local_into_string(x.clone(), ", ", config))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Record { cols, vals, .. } => cols
            .iter()
            .zip(vals.iter())
            .map(|(x, y)| format!("{}: {}", x, local_into_string(y.clone(), ", ", config)))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Block { val, .. } => format!("<Block {}>", val),
        Value::Closure { val, .. } => format!("<Closure {}>", val),
        Value::Nothing { .. } => String::new(),
        Value::Error { error } => format!("{:?}", error),
        Value::Binary { val, .. } => format!("{:?}", val),
        Value::CellPath { val, .. } => val.into_string(),
        Value::CustomValue { val, .. } => val.value_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToText {})
    }
}
