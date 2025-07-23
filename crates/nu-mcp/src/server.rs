use std::{borrow::Cow, sync::Arc};

use nu_protocol::{ShellError, Signature, Type, engine::EngineState};
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo, Tool},
    tool_handler, tool_router,
};
use schemars::{Schema, json_schema};
use serde_json::{self, json};

pub struct NushellMcpServer {
    engine_state: Arc<EngineState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(engine_state: EngineState) -> Self {
        NushellMcpServer {
            engine_state: Arc::new(engine_state),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl ServerHandler for NushellMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("generic data service".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn command_to_tool(command: &dyn nu_protocol::engine::Command) -> Result<Tool, ShellError> {
    let input_schema = json_schema_signature(&command.signature())?;
    Ok(Tool {
        name: Cow::Owned(command.name().to_owned()),
        description: Some(Cow::Owned(command.description().to_owned())),
        input_schema: Arc::new(rmcp::model::object(input_schema.into())),
        annotations: None,
    })
}

fn json_schema_signature(signature: &Signature) -> Result<Schema, ShellError> {
    if signature.input_output_types.len() == 1 {
        let (input_type, _) = signature.input_output_types[0].clone();
        Ok(
            Schema::try_from(json_schema_for_type(&input_type)?).map_err(|e| {
                ShellError::GenericError {
                    error: format!("failed to conver to schema: {e}"),
                    msg: "".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                }
            })?,
        )
    } else {
        let input_schemas: Vec<Schema> = signature
            .input_output_types
            .iter()
            .map(|(input_type, _)| json_schema_for_type(input_type))
            .map(|schema| schema.and_then(into_schema))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(json_schema!({
            "type": "object",
            "oneOf": input_schemas,
        }))
    }
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

fn json_schema_for_type(ty: &Type) -> Result<serde_json::Value, ShellError> {
    let schema = match ty {
        Type::Any => json!({
            "type": ["null", "boolean", "integer", "number", "string", "array", "object"]
        }),
        Type::Binary => json!({
            "type": "string",
            "format": "binary"
        }),
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

/// Convert a SyntaxShape to a JSON Schema type string
// fn syntax_shape_to_json_type(shape: &nu_protocol::SyntaxShape) -> String {
//     use nu_protocol::SyntaxShape;
//
//     match shape {
//         SyntaxShape::Int => "integer".to_string(),
//         SyntaxShape::Float | SyntaxShape::Number => "number".to_string(),
//         SyntaxShape::String | SyntaxShape::Filepath | SyntaxShape::GlobPattern => "string".to_string(),
//         SyntaxShape::Boolean => "boolean".to_string(),
//         SyntaxShape::Table | SyntaxShape::List => "array".to_string(),
//         SyntaxShape::Record | SyntaxShape::Any => "object".to_string(),
//         _ => "string".to_string(), // Default to string for other shapes
//     }
// }

#[cfg(test)]
mod tests {

    use jsonschema::{Draft, Validator};
    use serde_json::json;

    use super::*;

    #[test]
    fn test_schema_for_string_type() {
        let schema = json_schema_for_type(Type::String);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_int_type() {
        let schema = json_schema_for_type(Type::Int);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_float_type() {
        let schema = json_schema_for_type(Type::Float);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_bool_type() {
        let schema = json_schema_for_type(Type::Bool);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_any_type() {
        let schema = json_schema_for_type(Type::Any);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_binary_type() {
        let schema = json_schema_for_type(Type::Binary);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_date_type() {
        let schema = json_schema_for_type(Type::Date);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_duration_type() {
        let schema = json_schema_for_type(Type::Duration);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_filesize_type() {
        let schema = json_schema_for_type(Type::Filesize);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_nothing_type() {
        let schema = json_schema_for_type(Type::Nothing);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_number_type() {
        let schema = json_schema_for_type(Type::Number);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_range_type() {
        let schema = json_schema_for_type(Type::Range);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_list_type() {
        let schema = json_schema_for_type(Type::List(Box::new(Type::Int)));
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_record_type() {
        let fields =
            vec![("name".into(), Type::String), ("age".into(), Type::Int)].into_boxed_slice();
        let schema = json_schema_for_type(Type::Record(fields));
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_table_type() {
        let columns =
            vec![("name".into(), Type::String), ("age".into(), Type::Int)].into_boxed_slice();
        let schema = json_schema_for_type(Type::Table(columns));
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_custom_type() {
        let schema = json_schema_for_type(Type::Custom("MyType".into()));
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_glob_type() {
        let schema = json_schema_for_type(Type::Glob);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_error_type() {
        let schema = json_schema_for_type(Type::Error);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_nested_list_type() {
        let schema = json_schema_for_type(Type::List(Box::new(Type::List(Box::new(Type::String)))));
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_cellpath_type() {
        let schema = json_schema_for_type(Type::CellPath);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_block_type() {
        let schema = json_schema_for_type(Type::Block);
        validate_schema(schema);
    }

    #[test]
    fn test_schema_for_closure_type() {
        let schema = json_schema_for_type(Type::Closure);
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
