use nu_engine::command_prelude::*;
use nu_utils::JsonFlattener; // Ensure this import is present // Ensure this import is present

#[derive(Clone)]
pub struct ConfigFlatten;

impl Command for ConfigFlatten {
    fn name(&self) -> &str {
        "config flatten"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn description(&self) -> &str {
        "Show the current configuration in a flattened form."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the current configuration in a flattened form",
            example: "config flatten",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Get the Config instance from the EngineState
        let config = engine_state.get_config();
        // Serialize the Config instance to JSON
        let serialized_config =
            serde_json::to_value(&**config).map_err(|err| ShellError::GenericError {
                error: format!("Failed to serialize config to json: {err}"),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;
        // Create a JsonFlattener instance with appropriate arguments
        let flattener = JsonFlattener {
            separator: ".",
            alt_array_flattening: false,
            preserve_arrays: true,
        };
        // Flatten the JSON value
        let flattened_config_str = flattener.flatten(&serialized_config).to_string();
        let flattened_values = convert_string_to_value(&flattened_config_str, call.head)?;

        Ok(flattened_values.into_pipeline_data())
    }
}

// From here below is taken from `from json`. Would be nice to have a nu-utils-value crate that could be shared
fn convert_string_to_value(string_input: &str, span: Span) -> Result<Value, ShellError> {
    match nu_json::from_str(string_input) {
        Ok(value) => Ok(convert_nujson_to_value(value, span)),

        Err(x) => match x {
            nu_json::Error::Syntax(_, row, col) => {
                let label = x.to_string();
                let label_span = convert_row_column_to_span(row, col, string_input);
                Err(ShellError::GenericError {
                    error: "Error while parsing JSON text".into(),
                    msg: "error parsing JSON text".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![ShellError::OutsideSpannedLabeledError {
                        src: string_input.into(),
                        error: "Error while parsing JSON text".into(),
                        msg: label,
                        span: label_span,
                    }],
                })
            }
            x => Err(ShellError::CantConvert {
                to_type: format!("structured json data ({x})"),
                from_type: "string".into(),
                span,
                help: None,
            }),
        },
    }
}

fn convert_nujson_to_value(value: nu_json::Value, span: Span) -> Value {
    match value {
        nu_json::Value::Array(array) => Value::list(
            array
                .into_iter()
                .map(|x| convert_nujson_to_value(x, span))
                .collect(),
            span,
        ),
        nu_json::Value::Bool(b) => Value::bool(b, span),
        nu_json::Value::F64(f) => Value::float(f, span),
        nu_json::Value::I64(i) => Value::int(i, span),
        nu_json::Value::Null => Value::nothing(span),
        nu_json::Value::Object(k) => Value::record(
            k.into_iter()
                .map(|(k, v)| (k, convert_nujson_to_value(v, span)))
                .collect(),
            span,
        ),
        nu_json::Value::U64(u) => {
            if u > i64::MAX as u64 {
                Value::error(
                    ShellError::CantConvert {
                        to_type: "i64 sized integer".into(),
                        from_type: "value larger than i64".into(),
                        span,
                        help: None,
                    },
                    span,
                )
            } else {
                Value::int(u as i64, span)
            }
        }
        nu_json::Value::String(s) => Value::string(s, span),
    }
}

// Converts row+column to a Span, assuming bytes (1-based rows)
fn convert_row_column_to_span(row: usize, col: usize, contents: &str) -> Span {
    let mut cur_row = 1;
    let mut cur_col = 1;

    for (offset, curr_byte) in contents.bytes().enumerate() {
        if curr_byte == b'\n' {
            cur_row += 1;
            cur_col = 1;
        }
        if cur_row >= row && cur_col >= col {
            return Span::new(offset, offset);
        } else {
            cur_col += 1;
        }
    }

    Span::new(contents.len(), contents.len())
}
