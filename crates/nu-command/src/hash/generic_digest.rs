use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Span;
use nu_protocol::{Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};
use std::marker::PhantomData;

pub trait HashDigest: digest::Digest + Clone {
    fn name() -> &'static str;
    fn examples() -> Vec<Example>;
}

#[derive(Clone)]
pub struct GenericDigest<D: HashDigest> {
    name: String,
    usage: String,
    phantom: PhantomData<D>,
}

impl<D: HashDigest> Default for GenericDigest<D> {
    fn default() -> Self {
        Self {
            name: format!("hash {}", D::name()),
            usage: format!("Hash a value using the {} hash algorithm", D::name()),
            phantom: PhantomData,
        }
    }
}

pub(super) struct Arguments {
    pub(super) cell_paths: Option<Vec<CellPath>>,
    pub(super) binary: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl<D> Command for GenericDigest<D>
where
    D: HashDigest + Send + Sync + 'static,
    digest::Output<D>: core::fmt::LowerHex,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::String, Type::Binary),
            ])
            .switch(
                "binary",
                "Output binary instead of hexadecimal representation",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                format!("optionally {} hash data by cell path", D::name()),
            )
    }

    fn usage(&self) -> &str {
        &self.usage
    }

    fn examples(&self) -> Vec<Example> {
        D::examples()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let binary = call.has_flag("binary");
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments { binary, cell_paths };
        operate(
            action::<D>,
            args,
            input,
            call.head,
            engine_state.ctrlc.clone(),
        )
    }
}

pub(super) fn action<D>(input: &Value, args: &Arguments, _span: Span) -> Value
where
    D: HashDigest,
    digest::Output<D>: core::fmt::LowerHex,
{
    let (bytes, span) = match input {
        Value::String { val, span } => (val.as_bytes(), *span),
        Value::Binary { val, span } => (val.as_slice(), *span),
        other => {
            let span = match input.span() {
                Ok(span) => span,
                Err(error) => return Value::Error { error },
            };

            return Value::Error {
                error: ShellError::UnsupportedInput(
                    format!(
                        "Type `{}` is not supported for {} hashing input",
                        other.get_type(),
                        D::name()
                    ),
                    span,
                ),
            };
        }
    };

    let digest = D::digest(bytes);

    if args.binary {
        Value::Binary {
            val: digest.to_vec(),
            span,
        }
    } else {
        Value::String {
            val: format!("{:x}", digest),
            span,
        }
    }
}
