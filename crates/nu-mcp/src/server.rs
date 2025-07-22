use std::sync::Arc;

use nu_protocol::{Signature, SyntaxShape, Type, engine::EngineState};
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo, Tool},
    tool_handler, tool_router,
};
use schemars::{Schema, json_schema, schema_for};
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

fn command_to_tool(command: &dyn nu_protocol::engine::Command) -> Tool {
    // Tool {
    //     name: command.name().into(),
    //     description: Some(command.description().into()),
    //     input_schema: command.inp
    //     output_schema: command.output_schema().clone(),
    //     annotations: None,
    // }
    todo!("implement me")
}

fn json_schema_signature(
    signature: &Signature,
) -> Result<(Schema, Schema), Box<dyn std::error::Error>> {
    if signature.input_output_types.len() == 1 {
        let (input_type, output_type) = signature.input_output_types[0].clone();
        Ok((
            Schema::try_from(json_schema_for_type(&input_type))?,
            Schema::try_from(json_schema_for_type(&output_type))?,
        ))
    } else {
        let input_schemas: Vec<Schema> = signature
            .input_output_types
            .iter()
            .map(|(input_type, _)| json_schema_for_type(input_type))
            .map(Schema::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let output_schemas: Vec<Schema> = signature
            .input_output_types
            .iter()
            .map(|(_, output_type)| json_schema_for_type(output_type))
            .map(Schema::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((
            json_schema!({
                "type": "object",
                "oneOf": input_schemas,
            }),
            json_schema!({
                "type": "object",
                "oneOf": output_schemas,
            }),
        ))
    }
}

fn json_schema_for_type(ty: &Type) -> serde_json::Value {
    match ty {
        Type::Any => json!({
            "type": ["null", "boolean", "integer", "number", "string", "array", "object"]
        }),
        Type::Binary => json!({
            "type": "string",
            "format": "binary"
        }),
        Type::Block => unimplemented!("Nushell Block type is not supported in JSON Schema"),
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
                "items": inner_schema
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
    }
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
