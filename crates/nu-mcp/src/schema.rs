use nu_protocol::{PositionalArg, ShellError, Signature, SyntaxShape, Type};
use schemars::{Schema, json_schema};
use serde_json::json;

pub(crate) fn json_schema_signature(signature: &Signature) -> Result<Schema, ShellError> {
    let description = "Pipeline input for a nushell command";

    let pipeline_input_schema = if signature.input_output_types.len() == 1 {
        let (input_type, _) = signature.input_output_types[0].clone();
        let json = json_schema_for_type(&input_type)?;
        set_object_field(
            json,
            "description",
            serde_json::Value::String(description.to_string()),
        )
    } else {
        let input_schemas: Vec<Schema> = signature
            .input_output_types
            .iter()
            .map(|(input_type, _)| json_schema_for_type(input_type))
            .map(|schema| schema.and_then(into_schema))
            .collect::<Result<Vec<_>, _>>()?;

        // todo - I don't think this is quite right, it needs to support an array with these
        // potential values not just one of them
        json!({
            "type": "object",
            "description": description,
            "oneOf": input_schemas,
        })
    };

    let properties = signature
        .positional
        .iter()
        .map(|positional| {
            json_schema_for_positional(positional.clone())
                .map(|schema| (positional.name.to_string(), schema))
        })
        .collect::<Result<serde_json::Map<String, serde_json::Value>, ShellError>>()?;

    let properties = json!({
        "pipeline_input": pipeline_input_schema,
    });

    Ok(json_schema!({
        "type": "object",
        "properties": {
            "pipeline_input": properties,
        }
    }))
}

fn into_schema(value: serde_json::Value) -> Result<Schema, ShellError> {
    Schema::try_from(value).map_err(|e| ShellError::GenericError {
        error: format!("Failed to convert JSON value to schema: {e}"),
        msg: e.to_string(),
        span: None,
        help: None,
        inner: vec![],
    })
}

fn json_schema_for_positional(positional: PositionalArg) -> Result<serde_json::Value, ShellError> {
    let schema = json_schema_for_syntax_shape(&positional.shape)?;
    let schema = set_object_field(
        schema,
        "title",
        serde_json::Value::String(positional.name.to_string()),
    );
    let schema = set_object_field(
        schema,
        "description",
        serde_json::Value::String(positional.desc),
    );
    Ok(schema)
}

fn json_schema_for_syntax_shape(shape: &SyntaxShape) -> Result<serde_json::Value, ShellError> {
    let ty = shape.to_type();
    json_schema_for_type(&ty)
}

fn json_schema_for_type(ty: &Type) -> Result<serde_json::Value, ShellError> {
    let schema = match ty {
        Type::Any => json!({
            "type": ["null", "boolean", "integer", "number", "string", "array", "object"]
        }),
        // todo - this probably supportable
        Type::Binary => {
            return Err(ShellError::GenericError {
                error: "Nushell Binary type is not supported in JSON Schema".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            });
        }
        Type::Block => {
            return Err(ShellError::GenericError {
                error: "Nushell Block type is not supported in JSON Schema".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            });
        }
        Type::Bool => json!({
            "type": "boolean"
        }),
        Type::CellPath => json!({
            "type": "string",
            "description": "Cell path expression"
        }),
        Type::Closure => json!({
            "type": "string",
            "description": "Nushell closure"
        }),
        Type::Custom(name) => json!({
            "type": "object",
            "title": name,
            "description": format!("Custom type: {}", name)
        }),
        Type::Date => json!({
            "type": "string",
            "format": "date-time"
        }),
        Type::Duration => json!({
            "type": "integer",
            "description": "Duration value in nanoseconds"
        }),
        Type::Error => json!({
            "type": "object",
            "properties": {
                "error": {
                    "type": "string"
                }
            },
            "required": ["error"]
        }),
        Type::Filesize => json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "integer"
                },
                "unit": {
                    "type": "string"
                }
            },
            "required": ["value", "unit"]
        }),
        Type::Float => json!({
            "type": "number"
        }),
        Type::Int => json!({
            "type": "integer"
        }),
        Type::List(inner_type) => {
            let inner_schema = json_schema_for_type(inner_type);
            json!({
                "type": "array",
                "items": inner_schema?
            })
        }
        Type::Nothing => json!({
            "type": "null"
        }),
        Type::Number => json!({
            "type": ["integer", "number"]
        }),
        Type::Range => json!({
            "type": "object",
            "properties": {
                "start": { "type": ["integer", "number"] },
                "end": { "type": ["integer", "number"] },
                "inclusive": { "type": "boolean" }
            },
            "required": ["start", "end", "inclusive"]
        }),
        Type::Record(fields) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for (field_name, field_type) in fields.iter() {
                properties.insert(
                    field_name.clone(),
                    serde_json::to_value(json_schema_for_type(field_type)).unwrap(),
                );
                required.push(serde_json::Value::String(field_name.clone()));
            }

            json!({
                "type": "object",
                "properties": properties,
                "required": required
            })
        }
        Type::String => json!({
            "type": "string"
        }),
        Type::Glob => json!({
            "type": "string",
        }),
        Type::Table(columns) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for (col_name, col_type) in columns.iter() {
                properties.insert(
                    col_name.clone(),
                    serde_json::to_value(json_schema_for_type(col_type)).unwrap(),
                );
                required.push(serde_json::Value::String(col_name.clone()));
            }

            json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            })
        }
    };
    Ok(schema)
}

fn set_object_field(
    mut value: serde_json::Value,
    key: &str,
    val: serde_json::Value,
) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(key.to_string(), val);
    }
    value
}

#[cfg(test)]
mod tests {

    use jsonschema::Draft;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_schema_for_string_type() {
        let schema = json_schema_for_type(&Type::String).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_int_type() {
        let schema = json_schema_for_type(&Type::Int).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_float_type() {
        let schema = json_schema_for_type(&Type::Float).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_bool_type() {
        let schema = json_schema_for_type(&Type::Bool).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_any_type() {
        let schema = json_schema_for_type(&Type::Any).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    #[should_panic(expected = "binary type is not supported")]
    fn test_schema_for_binary_type() {
        let schema = json_schema_for_type(&Type::Binary).expect("binary type is not supported");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_date_type() {
        let schema = json_schema_for_type(&Type::Date).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_duration_type() {
        let schema = json_schema_for_type(&Type::Duration).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_filesize_type() {
        let schema = json_schema_for_type(&Type::Filesize).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_nothing_type() {
        let schema = json_schema_for_type(&Type::Nothing).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_number_type() {
        let schema = json_schema_for_type(&Type::Number).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_range_type() {
        let schema = json_schema_for_type(&Type::Range).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_list_type() {
        let schema =
            json_schema_for_type(&Type::List(Box::new(Type::Int))).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_record_type() {
        let fields =
            vec![("name".into(), Type::String), ("age".into(), Type::Int)].into_boxed_slice();
        let schema = json_schema_for_type(&Type::Record(fields)).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_table_type() {
        let columns =
            vec![("name".into(), Type::String), ("age".into(), Type::Int)].into_boxed_slice();
        let schema = json_schema_for_type(&Type::Table(columns)).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_custom_type() {
        let schema =
            json_schema_for_type(&Type::Custom("MyType".into())).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_glob_type() {
        let schema = json_schema_for_type(&Type::Glob).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_error_type() {
        let schema = json_schema_for_type(&Type::Error).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_nested_list_type() {
        let schema =
            json_schema_for_type(&Type::List(Box::new(Type::List(Box::new(Type::String)))))
                .expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_cellpath_type() {
        let schema = json_schema_for_type(&Type::CellPath).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    #[should_panic(expected = "block is not supported")]
    fn test_schema_for_block_type() {
        let schema = json_schema_for_type(&Type::Block).expect("block is not supported");
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_closure_type() {
        let schema = json_schema_for_type(&Type::Closure).expect("expected valid schema");
        validate_schema(schema);
    }

    #[test]
    #[should_panic]
    fn test_should_fail() {
        let json = json!([]);
        validate_schema(json);
    }

    fn validate_schema(schema: impl Into<serde_json::Value>) {
        jsonschema::options()
            .with_draft(Draft::Draft7)
            .should_ignore_unknown_formats(false)
            .build(&schema.into())
            .expect("Should be a valid schema");
    }
}
