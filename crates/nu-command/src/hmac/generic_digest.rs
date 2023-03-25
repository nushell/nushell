use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Span;
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use std::marker::PhantomData;

pub trait HmacDigest: digest::Mac + digest::KeyInit + Clone {
    fn name() -> &'static str;
    fn examples() -> Vec<Example<'static>>;
}

#[derive(Clone)]
pub struct GenericDigest<D: HmacDigest> {
    name: String,
    usage: String,
    phantom: PhantomData<D>,
}

impl<D: HmacDigest> Default for GenericDigest<D> {
    fn default() -> Self {
        Self {
            name: format!("hmac {}", D::name()),
            usage: format!(
                "Hmac a value with a secret key using the {} hash algorithm",
                D::name()
            ),
            phantom: PhantomData,
        }
    }
}

pub(super) struct Arguments<D> {
    pub(super) cell_paths: Option<Vec<CellPath>>,
    pub(super) binary: bool,
    pub(super) mac: D,
}

impl<D> CmdArgument for Arguments<D> {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl<D> Command for GenericDigest<D>
where
    D: HmacDigest + Send + Sync + 'static,
    digest::Output<D>: core::fmt::LowerHex,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Hash)
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::String, Type::Binary),
            ])
            .required_named("key", SyntaxShape::String, "secret key", None)
            .switch(
                "binary",
                "Output binary instead of hexadecimal representation",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                format!("optionally {} hmac data by cell path", D::name()),
            )
    }

    fn usage(&self) -> &str {
        &self.usage
    }

    fn examples(&self) -> Vec<Example<'static>> {
        D::examples()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let binary = call.has_flag("binary");
        let key: String =
            call.get_flag(engine_state, stack, "key")?
                .ok_or(ShellError::MissingParameter {
                    param_name: "key".to_string(),
                    span: call.head,
                })?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            binary,
            cell_paths,
            mac: <D as digest::Mac>::new_from_slice(key.as_bytes()).map_err(|e| {
                ShellError::IncorrectValue {
                    msg: e.to_string(),
                    span: call.head,
                }
            })?,
        };
        operate(
            action::<D>,
            args,
            input,
            call.head,
            engine_state.ctrlc.clone(),
        )
    }
}

pub(super) fn action<D>(input: &Value, args: &Arguments<D>, _span: Span) -> Value
where
    D: HmacDigest,
    digest::Output<D>: core::fmt::LowerHex,
{
    let (bytes, span) = match input {
        Value::String { val, span } => (val.as_bytes(), *span),
        Value::Binary { val, span } => (val.as_slice(), *span),
        // Propagate existing errors
        Value::Error { .. } => return input.clone(),
        other => {
            let span = match input.span() {
                Ok(span) => span,
                Err(error) => {
                    return Value::Error {
                        error: Box::new(error),
                    }
                }
            };

            return Value::Error {
                error: Box::new(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string or binary".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: span,
                    src_span: other.expect_span(),
                }),
            };
        }
    };

    let mut mac = args.mac.clone();
    mac.update(bytes);
    let digest = mac.finalize().into_bytes();

    if args.binary {
        Value::Binary {
            val: digest.to_vec(),
            span,
        }
    } else {
        Value::String {
            val: format!("{digest:x}"),
            span,
        }
    }
}
