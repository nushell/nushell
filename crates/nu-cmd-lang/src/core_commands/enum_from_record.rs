use nu_engine::command_prelude::*;
use nu_protocol::{CompareTypes, EnumValue, TypeDefKind, shell_error::generic::GenericError};

/// Rebuilds a value of a user-declared `enum` type from its base record.
///
/// This command is what `Type.from-record <record>` compiles to; the parser
/// emits calls to it. It is the inverse of how enum values serialize:
/// `{kind: <variant>, ...}` becomes an enum value again, with the variant
/// tag and payload validated against the declaration.
#[derive(Clone)]
pub struct EnumFromRecord;

impl Command for EnumFromRecord {
    fn name(&self) -> &str {
        "enum-from-record"
    }

    fn description(&self) -> &str {
        "Rebuild a declared enum value from its base record (internal: `Type.from-record` compiles to this)."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("enum-from-record")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required(
                "type_name",
                SyntaxShape::String,
                "Name of the declared enum type.",
            )
            .required(
                "record",
                SyntaxShape::Any,
                "The base record to rebuild from.",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let type_name: String = call.req(engine_state, stack, 0)?;
        let record: Value = call.req(engine_state, stack, 1)?;

        let Some(type_def) = engine_state.find_type_name(type_name.as_bytes(), &[]) else {
            return Err(GenericError::new(
                format!("unknown type `{type_name}`"),
                "no `type` declaration with this name is in scope",
                call.head,
            )
            .into());
        };

        let TypeDefKind::Enum(enum_def) = &type_def.kind else {
            return Err(GenericError::new(
                format!("`{type_name}` is not an enum type"),
                "only enum types can be rebuilt from base records",
                call.head,
            )
            .into());
        };

        let enum_name = String::from_utf8_lossy(&type_def.name).to_string();

        let Value::Record { val: record, .. } = record else {
            return Err(ShellError::CantConvert {
                to_type: enum_name.clone(),
                from_type: record.get_type().to_string(),
                span: record.span(),
                help: Some(format!(
                    "`{type_name}.from-record` expects a record like {{kind: \"<variant>\", ...}}"
                )),
            });
        };
        let mut record = record.into_owned();

        let Some(kind) = record.remove("kind") else {
            return Err(GenericError::new(
                format!("missing `kind` field for `{enum_name}`"),
                "the base record needs a `kind` field naming the variant",
                call.head,
            )
            .into());
        };

        let variant_name = match kind {
            Value::String { val, .. } => val,
            other => {
                return Err(GenericError::new(
                    "`kind` must be a string naming the variant",
                    format!("got {}", other.get_type()),
                    other.span(),
                )
                .into());
            }
        };

        let Some(variant) = enum_def.get_variant(&variant_name) else {
            return Err(GenericError::new(
                format!("unknown variant `{variant_name}`"),
                format!("`{enum_name}` has no variant `{variant_name}`"),
                call.head,
            )
            .with_help(format!(
                "variants of `{enum_name}`: {}",
                enum_def.variant_names().join(", ")
            ))
            .into());
        };

        let payload = match &variant.payload {
            Some(shape) => {
                // The base record is uniformly `{kind, payload}`: the
                // payload is always a nested value under `payload`.
                let Some(payload) = record.remove("payload") else {
                    return Err(GenericError::new(
                        format!("missing `payload` field for `{enum_name}.{variant_name}`"),
                        format!("`{variant_name}` takes a {} payload", shape.to_type()),
                        call.head,
                    )
                    .into());
                };
                if !record.is_empty() {
                    return Err(GenericError::new(
                        format!("extra fields in base record for `{enum_name}.{variant_name}`"),
                        "only `kind` and `payload` are allowed here",
                        call.head,
                    )
                    .into());
                }

                let expected = shape.to_type();
                if !payload.get_type().is_subtype_of(&expected) {
                    return Err(ShellError::CantConvert {
                        to_type: expected.to_string(),
                        from_type: payload.get_type().to_string(),
                        span: call.head,
                        help: Some(format!(
                            "variant `{variant_name}` of `{enum_name}` expects {expected}"
                        )),
                    });
                }
                Some(payload)
            }
            None => {
                if !record.is_empty() {
                    return Err(GenericError::new(
                        format!("`{enum_name}.{variant_name}` takes no payload"),
                        "extra fields in the base record",
                        call.head,
                    )
                    .into());
                }
                None
            }
        };

        Ok(Value::custom(
            Box::new(EnumValue::new(enum_name, variant_name, payload)),
            call.head,
        )
        .into_pipeline_data())
    }
}
