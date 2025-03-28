use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;
use std::{io::Write, marker::PhantomData};

pub trait HashDigest: digest::Digest + Clone {
    fn name() -> &'static str;
    fn examples() -> Vec<Example<'static>>;
}

#[derive(Clone)]
pub struct GenericDigest<D: HashDigest> {
    name: String,
    description: String,
    phantom: PhantomData<D>,
}

impl<D: HashDigest> Default for GenericDigest<D> {
    fn default() -> Self {
        Self {
            name: format!("hash {}", D::name()),
            description: format!("Hash a value using the {} hash algorithm.", D::name()),
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
    D: HashDigest + Write + Send + Sync + 'static,
    digest::Output<D>: core::fmt::LowerHex,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Hash)
            .input_output_types(vec![
                (Type::String, Type::Any),
                (Type::Binary, Type::Any),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
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
                format!("Optionally {} hash data by cell path.", D::name()),
            )
    }

    fn description(&self) -> &str {
        &self.description
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
        let head = call.head;
        let binary = call.has_flag(engine_state, stack, "binary")?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let mut hasher = D::new();

        if let PipelineData::ByteStream(stream, ..) = input {
            stream.write_to(&mut hasher)?;
            let digest = hasher.finalize();
            if binary {
                Ok(Value::binary(digest.to_vec(), head).into_pipeline_data())
            } else {
                Ok(Value::string(format!("{digest:x}"), head).into_pipeline_data())
            }
        } else {
            let args = Arguments { binary, cell_paths };
            operate(action::<D>, args, input, head, engine_state.signals())
        }
    }
}

pub(super) fn action<D>(input: &Value, args: &Arguments, _span: Span) -> Value
where
    D: HashDigest,
    digest::Output<D>: core::fmt::LowerHex,
{
    let span = input.span();
    let (bytes, span) = match input {
        Value::String { val, .. } => (val.as_bytes(), span),
        Value::Binary { val, .. } => (val.as_slice(), span),
        // Propagate existing errors
        Value::Error { .. } => return input.clone(),
        other => {
            let span = input.span();

            return Value::error(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string or binary".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: span,
                    src_span: other.span(),
                },
                span,
            );
        }
    };

    let digest = D::digest(bytes);

    if args.binary {
        Value::binary(digest.to_vec(), span)
    } else {
        Value::string(format!("{digest:x}"), span)
    }
}
