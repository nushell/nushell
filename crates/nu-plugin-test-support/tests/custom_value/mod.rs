use std::cmp::Ordering;

use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, SimplePluginCommand};
use nu_plugin_test_support::PluginTest;
use nu_protocol::{
    CustomValue, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    Type, Value,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
struct CustomU32(u32);

impl CustomU32 {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom_value(Box::new(self), span)
    }
}

#[typetag::serde]
impl CustomValue for CustomU32 {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "CustomU32".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        other
            .as_custom_value()
            .ok()
            .and_then(|cv| cv.as_any().downcast_ref::<CustomU32>())
            .and_then(|other_u32| PartialOrd::partial_cmp(self, other_u32))
    }
}

struct CustomU32Plugin;
struct IntoU32;
struct IntoIntFromU32;

impl Plugin for CustomU32Plugin {
    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![Box::new(IntoU32), Box::new(IntoIntFromU32)]
    }
}

impl SimplePluginCommand for IntoU32 {
    type Plugin = CustomU32Plugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("into u32")
            .input_output_type(Type::Int, Type::Custom("CustomU32".into()))
            .plugin_examples(vec![PluginExample {
                example: "340 | into u32".into(),
                description: "Make a u32".into(),
                result: Some(CustomU32(340).into_value(Span::test_data())),
            }])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let value: i64 = input.as_int()?;
        let value_u32 = u32::try_from(value).map_err(|err| {
            LabeledError::new(format!("Not a valid u32: {value}"))
                .with_label(err.to_string(), input.span())
        })?;
        Ok(CustomU32(value_u32).into_value(call.head))
    }
}

impl SimplePluginCommand for IntoIntFromU32 {
    type Plugin = CustomU32Plugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("into int from u32")
            .input_output_type(Type::Custom("CustomU32".into()), Type::Int)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let value: &CustomU32 = input
            .as_custom_value()?
            .as_any()
            .downcast_ref()
            .ok_or_else(|| ShellError::TypeMismatch {
                err_message: "expected CustomU32".into(),
                span: input.span(),
            })?;
        Ok(Value::int(value.0 as i64, call.head))
    }
}

#[test]
fn test_into_u32_examples() -> Result<(), ShellError> {
    PluginTest::new("custom_u32", CustomU32Plugin.into())?.test_command_examples(&IntoU32)
}

#[test]
fn test_into_int_from_u32() -> Result<(), ShellError> {
    let result = PluginTest::new("custom_u32", CustomU32Plugin.into())?
        .eval_with(
            "into int from u32",
            PipelineData::Value(CustomU32(42).into_value(Span::test_data()), None),
        )?
        .into_value(Span::test_data());
    assert_eq!(Value::test_int(42), result);
    Ok(())
}
