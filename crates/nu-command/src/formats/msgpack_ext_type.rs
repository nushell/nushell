use nu_engine::CallExt;
use nu_protocol::{
    Category, CustomValue, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature,
    Span, SyntaxShape, Type, Value,
    casing::Casing,
    engine::{Call, Command, EngineState, Stack},
};
use nu_utils::IgnoreCaseExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgpackExtensionType {
    pub ty: i8,
    pub data: Vec<u8>,
}

impl MsgpackExtensionType {
    fn ty(&self, span: Span) -> Value {
        Value::int(self.ty.into(), span)
    }

    fn data(&self, span: Span) -> Value {
        Value::binary(self.data.clone(), span)
    }
}

#[typetag::serde]
impl CustomValue for MsgpackExtensionType {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        std::any::type_name::<Self>().to_owned()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::record(
            Record::from_iter([
                ("ty".to_owned(), self.ty(span)),
                ("data".to_owned(), self.data(span)),
            ]),
            span,
        ))
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        match (&*column_name, casing) {
            ("ty", _) => Ok(self.ty(self_span)),
            ("data", _) => Ok(self.data(self_span)),

            (str, Casing::Insensitive) if str.eq_ignore_case("ty") => Ok(self.ty(self_span)),
            (str, Casing::Insensitive) if str.eq_ignore_case("data") => Ok(self.data(self_span)),

            _ if optional => Ok(Value::nothing(path_span)),
            _ => Err(ShellError::IncompatiblePathAccess {
                type_name: self.type_name(),
                span: path_span,
            }),
        }
    }

    fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.data.len()
    }

    /// Any representation used to downcast object to its original type
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Any representation used to downcast object to its original type (mutable reference)
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone)]
pub struct MsgpackExt;

impl Command for MsgpackExt {
    fn name(&self) -> &str {
        "msgpack ext"
    }

    fn signature(&self) -> Signature {
        Signature::build("msgpack ext")
            .input_output_types(vec![(
                Type::Binary,
                Type::Custom(std::any::type_name::<MsgpackExtensionType>().into()),
            )])
            .required("type", SyntaxShape::Int, "Extension type to use")
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Create msgpack extension type.\nSee <https://github.com/msgpack/msgpack/blob/master/spec.md#extension-types>"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        // TODO: add some examples
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.take_metadata().unwrap_or_default();
        let input_span = input.span().unwrap_or(call.head);
        let data = input.into_value(input_span)?.into_binary()?;

        let ty = call.req(engine_state, stack, 0)?;
        Ok(
            Value::custom(Box::new(MsgpackExtensionType { ty, data }), call.head)
                .into_pipeline_data_with_metadata(metadata),
        )
    }
}
