use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;
use std::fmt::Write as _;

/// YAML 1.1 boolean-like strings that need quoting when used as record keys.
const YAML_11_BOOLEANS: &[&str] = &[
    "y", "Y", "yes", "Yes", "YES", "n", "N", "no", "No", "NO", "on", "On", "ON", "off", "Off",
    "OFF",
];

/// YAML special float and numeric strings that need quoting to preserve them as strings.
const YAML_SPECIAL_NUMERICS: &[&str] = &[
    ".inf", ".Inf", ".INF", "-.inf", "-.Inf", "-.INF", ".nan", ".NaN", ".NAN",
];

#[derive(Clone)]
pub struct ToYamlLike(&'static str);
pub const TO_YAML: ToYamlLike = ToYamlLike("to yaml");
pub const TO_YML: ToYamlLike = ToYamlLike("to yml");

impl Command for ToYamlLike {
    fn name(&self) -> &str {
        self.0
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into .yaml/.yml text."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Outputs a YAML string representing the contents of this table.",
            example: match self.name() {
                "to yaml" => r#"[[foo bar]; ["1" "2"]] | to yaml"#,
                "to yml" => r#"[[foo bar]; ["1" "2"]] | to yml"#,
                _ => unreachable!("only implemented for `yaml` and `yml`"),
            },
            result: Some(Value::test_string("- foo: '1'\n  bar: '2'\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;
        let input = input.try_expand_range()?;

        to_yaml(engine_state, input, head, serialize_types)
    }
}

pub fn value_to_yaml_value(
    engine_state: &EngineState,
    v: &Value,
    serialize_types: bool,
) -> Result<serde_yaml::Value, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => serde_yaml::Value::Bool(*val),
        Value::Int { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        Value::Filesize { val, .. } => {
            serde_yaml::Value::Number(serde_yaml::Number::from(val.get()))
        }
        Value::Duration { val, .. } => serde_yaml::Value::String(val.to_string()),
        Value::Date { val, .. } => serde_yaml::Value::String(val.to_string()),
        Value::Range { .. } => serde_yaml::Value::Null,
        Value::Float { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        Value::String { val, .. } | Value::Glob { val, .. } => {
            serde_yaml::Value::String(val.clone())
        }
        Value::Record { val, .. } => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in &**val {
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml_value(engine_state, v, serialize_types)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
        Value::List { vals, .. } => {
            let mut out = vec![];

            for value in vals {
                out.push(value_to_yaml_value(engine_state, value, serialize_types)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Closure { val, .. } => {
            if serialize_types {
                let block = engine_state.get_block(val.block_id);
                if let Some(span) = block.span {
                    let contents_bytes = engine_state.get_span_contents(span);
                    let contents_string = String::from_utf8_lossy(contents_bytes);
                    serde_yaml::Value::String(contents_string.to_string())
                } else {
                    serde_yaml::Value::String(format!(
                        "unable to retrieve block contents for yaml block_id {}",
                        val.block_id.get()
                    ))
                }
            } else {
                serde_yaml::Value::Null
            }
        }
        Value::Nothing { .. } => serde_yaml::Value::Null,
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Binary { val, .. } => serde_yaml::Value::Sequence(
            val.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        Value::CellPath { val, .. } => serde_yaml::Value::Sequence(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(serde_yaml::Value::String(val.clone())),
                    PathMember::Int { val, .. } => {
                        Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*val)))
                    }
                })
                .collect::<Result<Vec<serde_yaml::Value>, ShellError>>()?,
        ),
        Value::Custom { .. } => serde_yaml::Value::Null,
    })
}

fn render_yaml_string(value: &str) -> String {
    if value.chars().any(char::is_control) {
        let mut escaped = String::with_capacity(value.len() + 2);
        escaped.push('"');

        for ch in value.chars() {
            match ch {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                '\u{08}' => escaped.push_str("\\b"),
                '\u{0C}' => escaped.push_str("\\f"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                c if c.is_control() => {
                    let _ = write!(escaped, "\\u{:04X}", c as u32);
                }
                c => escaped.push(c),
            }
        }

        escaped.push('"');
        escaped
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn should_quote_yaml_key(key: &str) -> bool {
    if key.is_empty() {
        return true;
    }
    if key.chars().any(char::is_control) {
        return true;
    }
    if key.starts_with(char::is_whitespace) || key.ends_with(char::is_whitespace) {
        return true;
    }
    if YAML_11_BOOLEANS.contains(&key) {
        return true;
    }
    if matches!(
        key,
        "~" | "null" | "Null" | "NULL" | "true" | "True" | "TRUE" | "false" | "False" | "FALSE"
    ) {
        return true;
    }
    // Check for YAML special numeric values (.inf, .nan) and hex/octal notation
    if YAML_SPECIAL_NUMERICS.contains(&key) {
        return true;
    }
    if key.starts_with("0x") || key.starts_with("0X") {
        return true;
    }
    if key.starts_with("0o") || key.starts_with("0O") {
        return true;
    }
    if key.parse::<i64>().is_ok() {
        return true;
    }
    if key.parse::<u64>().is_ok() {
        return true;
    }
    if key.parse::<f64>().is_ok() {
        return true;
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
    {
        return true;
    }
    false
}

fn render_yaml_key(key: &serde_yaml::Value) -> String {
    match key {
        serde_yaml::Value::String(key) if should_quote_yaml_key(key) => render_yaml_string(key),
        serde_yaml::Value::String(key) => key.clone(),
        _ => render_inline_yaml_value(key),
    }
}

fn render_inline_yaml_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => "null".to_string(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => render_yaml_string(value),
        serde_yaml::Value::Sequence(values) => {
            let values = values
                .iter()
                .map(render_inline_yaml_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{values}]")
        }
        serde_yaml::Value::Mapping(entries) => {
            let entries = entries
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{}: {}",
                        render_yaml_key(key),
                        render_inline_yaml_value(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{entries}}}")
        }
        serde_yaml::Value::Tagged(tagged) => {
            format!("{} {}", tagged.tag, render_inline_yaml_value(&tagged.value))
        }
    }
}

fn is_inline_yaml_value(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::Sequence(values) => values.is_empty(),
        serde_yaml::Value::Mapping(entries) => entries.is_empty(),
        serde_yaml::Value::Tagged(tagged) => is_inline_yaml_value(&tagged.value),
        _ => true,
    }
}

fn write_yaml_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

fn write_yaml_value(output: &mut String, value: &serde_yaml::Value, indent: usize) {
    match value {
        serde_yaml::Value::Sequence(values) if !values.is_empty() => {
            write_yaml_sequence(output, values, indent);
        }
        serde_yaml::Value::Mapping(entries) if !entries.is_empty() => {
            write_yaml_mapping(output, entries, indent, "");
        }
        serde_yaml::Value::Tagged(tagged) => write_yaml_value(output, &tagged.value, indent),
        _ => {
            write_yaml_indent(output, indent);
            output.push_str(&render_inline_yaml_value(value));
            output.push('\n');
        }
    }
}

fn write_yaml_sequence(output: &mut String, values: &[serde_yaml::Value], indent: usize) {
    for value in values {
        match value {
            serde_yaml::Value::Mapping(entries) if !entries.is_empty() => {
                write_yaml_mapping(output, entries, indent, "- ");
            }
            value if is_inline_yaml_value(value) => {
                write_yaml_indent(output, indent);
                output.push_str("- ");
                output.push_str(&render_inline_yaml_value(value));
                output.push('\n');
            }
            _ => {
                write_yaml_indent(output, indent);
                output.push_str("-\n");
                write_yaml_value(output, value, indent + 2);
            }
        }
    }
}

fn write_yaml_mapping(
    output: &mut String,
    entries: &serde_yaml::Mapping,
    indent: usize,
    first_prefix: &str,
) {
    let first_prefix_len = first_prefix.len();

    for (index, (key, value)) in entries.iter().enumerate() {
        let is_first = index == 0;
        // For the first entry, the prefix is written at the current indent level.
        // For subsequent entries, we need to account for the prefix's length
        // to maintain proper alignment with the first entry.
        let line_indent = indent + if is_first { 0 } else { first_prefix_len };
        // The key starts after the prefix (already written for first entry,
        // will be written as part of indent for subsequent entries).
        let key_indent = line_indent + if is_first { first_prefix_len } else { 0 };

        write_yaml_indent(output, line_indent);
        if is_first {
            output.push_str(first_prefix);
        }

        output.push_str(&render_yaml_key(key));

        if is_inline_yaml_value(value) {
            output.push_str(": ");
            output.push_str(&render_inline_yaml_value(value));
            output.push('\n');
        } else {
            output.push_str(":\n");
            write_yaml_value(output, value, key_indent + 2);
        }
    }
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    let mut output = String::new();
    write_yaml_value(&mut output, value, 0);
    output
}

fn to_yaml(
    engine_state: &EngineState,
    mut input: PipelineData,
    head: Span,
    serialize_types: bool,
) -> Result<PipelineData, ShellError> {
    let metadata = input
        .take_metadata()
        .unwrap_or_default()
        // Per RFC-9512, application/yaml should be used
        .with_content_type(Some("application/yaml".into()));
    let value = input.into_value(head)?;

    let yaml_value = value_to_yaml_value(engine_state, &value, serialize_types)?;
    let yaml_string = yaml_value_to_string(&yaml_value);
    Ok(Value::string(yaml_string, head).into_pipeline_data_with_metadata(Some(metadata)))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Get, Metadata};
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(TO_YAML)?;
        nu_test_support::test().examples(TO_YML)
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(TO_YAML));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to yaml  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/yaml"),
            result.expect("There should be a result")
        );
    }
}
