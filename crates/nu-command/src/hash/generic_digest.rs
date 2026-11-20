use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use std::{
    fmt::Write as _,
    io::{self, Write},
    marker::PhantomData,
};

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

/// Feeds a byte stream into a digest one chunk at a time.
///
/// `digest` 0.11 dropped the `io::Write` impl on hash types, so hashing a
/// [`ByteStream`](nu_protocol::ByteStream) needs this bridge onto `update` to
/// stay within a fixed amount of memory.
struct DigestWriter<'a, D: HashDigest>(&'a mut D);

impl<D: HashDigest> Write for DigestWriter<'_, D> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

impl<D> Command for GenericDigest<D>
where
    D: HashDigest + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Hash)
            .input_output_types(vec![
                (Type::String, Type::one_of([Type::String, Type::Binary])),
                (Type::Binary, Type::one_of([Type::String, Type::Binary])),
                (
                    Type::list(Type::String),
                    Type::list(Type::one_of([Type::String, Type::Binary])),
                ),
                (
                    Type::list(Type::Binary),
                    Type::list(Type::one_of([Type::String, Type::Binary])),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "binary",
                "Output binary instead of hexadecimal representation.",
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

        if let PipelineData::ByteStream(stream, ..) = input {
            let mut hasher = D::new();
            stream.write_to(DigestWriter(&mut hasher))?;
            let digest = hasher.finalize();
            if binary {
                Ok(
                    Value::binary(<[u8]>::to_vec(AsRef::<[u8]>::as_ref(&digest)), head)
                        .into_pipeline_data(),
                )
            } else {
                Ok(Value::string(hex_encode(digest.as_ref()), head).into_pipeline_data())
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
        Value::binary(<[u8]>::to_vec(AsRef::<[u8]>::as_ref(&digest)), span)
    } else {
        Value::string(hex_encode(AsRef::<[u8]>::as_ref(&digest)), span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use digest::Digest;
    use sha2::Sha256;

    /// A stream must hash the same as the bytes it carries, whatever the
    /// chunking — the digest is fed incrementally, never collected.
    #[test]
    fn chunked_writes_match_one_shot() {
        let data: Vec<u8> = (0..=u8::MAX).cycle().take(300 * 1024).collect();

        let mut hasher = Sha256::new();
        io::copy(&mut data.as_slice(), &mut DigestWriter(&mut hasher)).unwrap();

        assert_eq!(hasher.finalize(), Sha256::digest(&data));
    }
}
