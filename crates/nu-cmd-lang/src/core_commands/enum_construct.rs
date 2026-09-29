use nu_engine::command_prelude::*;
use nu_protocol::{CompareTypes, EnumValue, TypeDefKind, shell_error::generic::GenericError};

/// Constructs a value of a user-declared `enum` type.
///
/// This command is what `Type.variant [payload]` compiles to; the parser emits
/// calls to it. Calling it directly works too, but the declared enum type must
/// exist in scope.
#[derive(Clone)]
pub struct EnumConstruct;

impl Command for EnumConstruct {
    fn name(&self) -> &str {
        "enum-construct"
    }

    fn description(&self) -> &str {
        "Construct a value of a declared enum type (internal: `Type.variant` compiles to this)."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("enum-construct")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required(
                "type_name",
                SyntaxShape::String,
                "Name of the declared enum type.",
            )
            .required("variant", SyntaxShape::String, "Variant to construct.")
            .optional("payload", SyntaxShape::Any, "Variant payload.")
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
        let variant_name: String = call.req(engine_state, stack, 1)?;
        let payload: Option<Value> = call.opt(engine_state, stack, 2)?;

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
                "only enum types have variants to construct",
                call.head,
            )
            .into());
        };

        let Some(variant) = enum_def.get_variant(&variant_name) else {
            return Err(GenericError::new(
                format!("unknown variant `{variant_name}`"),
                format!("`{type_name}` has no variant `{variant_name}`"),
                call.head,
            )
            .with_help(format!(
                "variants of `{type_name}`: {}",
                enum_def.variant_names().join(", ")
            ))
            .into());
        };

        // The canonical declared name — the `type_name` argument may be a
        // module-qualified path like `mod.T`, but values report `T`.
        let enum_name = String::from_utf8_lossy(&type_def.name).to_string();

        match (&variant.payload, payload) {
            (Some(shape), Some(payload)) => {
                let expected = shape.to_type();
                if !payload.get_type().is_subtype_of(&expected) {
                    return Err(ShellError::CantConvert {
                        to_type: expected.to_string(),
                        from_type: payload.get_type().to_string(),
                        span: payload.span(),
                        help: Some(format!(
                            "variant `{variant_name}` of `{type_name}` expects {expected}"
                        )),
                    });
                }
                Ok(Value::custom(
                    Box::new(EnumValue::new(enum_name, variant_name, Some(payload))),
                    call.head,
                )
                .into_pipeline_data())
            }
            (Some(shape), None) => Err(ShellError::MissingParameter {
                param_name: format!("{type_name}.{variant_name} <{}>", shape.to_type()),
                span: call.head,
            }),
            (None, Some(payload)) => Err(GenericError::new(
                format!("`{type_name}.{variant_name}` takes no payload"),
                "unexpected payload",
                payload.span(),
            )
            .into()),
            (None, None) => Ok(Value::custom(
                Box::new(EnumValue::new(enum_name, variant_name, None)),
                call.head,
            )
            .into_pipeline_data()),
        }
    }
}
