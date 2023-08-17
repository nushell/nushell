use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SpannedValue, SyntaxShape, Type,
};
use nu_protocol::{IntoPipelineData, Span};
use std::marker::PhantomData;

pub trait HashDigest: digest::Digest + Clone {
    fn name() -> &'static str;
    fn examples() -> Vec<Example<'static>>;
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
            .category(Category::Hash)
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::String, Type::Binary),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments { binary, cell_paths };
        let mut hasher = D::new();
        match input {
            PipelineData::ExternalStream {
                stdout: Some(stream),
                span,
                ..
            } => {
                for item in stream {
                    match item {
                        // String and binary data are valid byte patterns
                        Ok(SpannedValue::String { val, .. }) => hasher.update(val.as_bytes()),
                        Ok(SpannedValue::Binary { val, .. }) => hasher.update(val),
                        // If any Error value is output, echo it back
                        Ok(v @ SpannedValue::Error { .. }) => return Ok(v.into_pipeline_data()),
                        // Unsupported data
                        Ok(other) => {
                            return Ok(SpannedValue::Error {
                                error: Box::new(ShellError::OnlySupportsThisInputType {
                                    exp_input_type: "string and binary".into(),
                                    wrong_type: other.get_type().to_string(),
                                    dst_span: span,
                                    src_span: other.expect_span(),
                                }),
                            }
                            .into_pipeline_data());
                        }
                        Err(err) => return Err(err),
                    };
                }
                let digest = hasher.finalize();
                if args.binary {
                    Ok(SpannedValue::Binary {
                        val: digest.to_vec(),
                        span,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(SpannedValue::String {
                        val: format!("{digest:x}"),
                        span,
                    }
                    .into_pipeline_data())
                }
            }
            _ => operate(
                action::<D>,
                args,
                input,
                call.head,
                engine_state.ctrlc.clone(),
            ),
        }
    }
}

pub(super) fn action<D>(input: &SpannedValue, args: &Arguments, _span: Span) -> SpannedValue
where
    D: HashDigest,
    digest::Output<D>: core::fmt::LowerHex,
{
    let (bytes, span) = match input {
        SpannedValue::String { val, span } => (val.as_bytes(), *span),
        SpannedValue::Binary { val, span } => (val.as_slice(), *span),
        // Propagate existing errors
        SpannedValue::Error { .. } => return input.clone(),
        other => {
            let span = match input.span() {
                Ok(span) => span,
                Err(error) => {
                    return SpannedValue::Error {
                        error: Box::new(error),
                    }
                }
            };

            return SpannedValue::Error {
                error: Box::new(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string or binary".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: span,
                    src_span: other.expect_span(),
                }),
            };
        }
    };

    let digest = D::digest(bytes);

    if args.binary {
        SpannedValue::Binary {
            val: digest.to_vec(),
            span,
        }
    } else {
        SpannedValue::String {
            val: format!("{digest:x}"),
            span,
        }
    }
}
