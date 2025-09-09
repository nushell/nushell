use nu_engine::command_prelude::*;
use oem_cp::decode_string_complete_table;
use std::collections::HashMap;
use std::sync::LazyLock;

// create a lazycell of all the code_table "Complete" code pages
// the commented out code pages are "Incomplete", which means they
// are stored as Option<char> and not &[char; 128]
static OEM_DECODE: LazyLock<HashMap<usize, &[char; 128]>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(437, &oem_cp::code_table::DECODING_TABLE_CP437);
    // m.insert(720, &oem_cp::code_table::DECODING_TABLE_CP720);
    m.insert(737, &oem_cp::code_table::DECODING_TABLE_CP737);
    m.insert(775, &oem_cp::code_table::DECODING_TABLE_CP775);

    m.insert(850, &oem_cp::code_table::DECODING_TABLE_CP850);
    m.insert(852, &oem_cp::code_table::DECODING_TABLE_CP852);
    m.insert(855, &oem_cp::code_table::DECODING_TABLE_CP855);
    // m.insert(857, &oem_cp::code_table::DECODING_TABLE_CP857);
    m.insert(858, &oem_cp::code_table::DECODING_TABLE_CP858);
    m.insert(860, &oem_cp::code_table::DECODING_TABLE_CP860);
    m.insert(861, &oem_cp::code_table::DECODING_TABLE_CP861);
    m.insert(862, &oem_cp::code_table::DECODING_TABLE_CP862);
    m.insert(863, &oem_cp::code_table::DECODING_TABLE_CP863);
    // m.insert(864, &oem_cp::code_table::DECODING_TABLE_CP864);
    m.insert(865, &oem_cp::code_table::DECODING_TABLE_CP865);
    m.insert(866, &oem_cp::code_table::DECODING_TABLE_CP866);
    // m.insert(869, &oem_cp::code_table::DECODING_TABLE_CP869);
    // m.insert(874, &oem_cp::code_table::DECODING_TABLE_CP874);

    m
});

#[derive(Clone)]
pub struct Decode;

impl Command for Decode {
    fn name(&self) -> &str {
        "decode"
    }

    fn description(&self) -> &str {
        "Decode bytes into a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "encoding", "decoding"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("decode")
            .input_output_types(vec![(Type::Binary, Type::String)])
            .optional("encoding", SyntaxShape::String, "The text encoding to use.")
            .category(Category::Strings)
    }

    fn extra_description(&self) -> &str {
        r#"Multiple encodings are supported; here are a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/latest/encoding_rs/#statics"#
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode the output of an external command",
                example: "^cat myfile.q | decode utf-8",
                result: None,
            },
            Example {
                description: "Decode an UTF-16 string into nushell UTF-8 string",
                example: r#"0x[00 53 00 6F 00 6D 00 65 00 20 00 44 00 61 00 74 00 61] | decode utf-16be"#,
                result: Some(Value::string("Some Data".to_owned(), Span::test_data())),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        run(call, input, encoding)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let encoding: Option<Spanned<String>> = call.opt_const(working_set, 0)?;
        run(call, input, encoding)
    }
}

fn run(
    call: &Call,
    input: PipelineData,
    encoding: Option<Spanned<String>>,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    match input {
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            let bytes = stream.into_bytes()?;
            match encoding {
                Some(encoding_name) => detect_and_decode(encoding_name, head, bytes),
                None => super::encoding::detect_encoding_name(head, span, &bytes)
                    .map(|encoding| encoding.decode(&bytes).0.into_owned())
                    .map(|s| Value::string(s, head)),
            }
            .map(|val| val.into_pipeline_data())
        }
        PipelineData::Value(v, ..) => {
            let input_span = v.span();
            match v {
                Value::Binary { val: bytes, .. } => match encoding {
                    Some(encoding_name) => detect_and_decode(encoding_name, head, bytes),
                    None => super::encoding::detect_encoding_name(head, input_span, &bytes)
                        .map(|encoding| encoding.decode(&bytes).0.into_owned())
                        .map(|s| Value::string(s, head)),
                }
                .map(|val| val.into_pipeline_data()),
                Value::Error { error, .. } => Err(*error),
                _ => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "binary".into(),
                    wrong_type: v.get_type().to_string(),
                    dst_span: head,
                    src_span: v.span(),
                }),
            }
        }
        // This should be more precise, but due to difficulties in getting spans
        // from PipelineData::ListData, this is as it is.
        _ => Err(ShellError::UnsupportedInput {
            msg: "non-binary input".into(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: input.span().unwrap_or(head),
        }),
    }
}

// Since we have two different decoding mechanisms, we allow oem_cp to be
// specified by only a number like `open file | decode 850`. If this decode
// parameter parses as a usize then we assume it was intentional and use oem_cp
// crate. Otherwise, if it doesn't parse as a usize, we assume it was a string
// and use the encoding_rs crate to try and decode it.
fn detect_and_decode(
    encoding_name: Spanned<String>,
    head: Span,
    bytes: Vec<u8>,
) -> Result<Value, ShellError> {
    let dec_table_id = encoding_name.item.parse::<usize>().unwrap_or(0usize);
    if dec_table_id == 0 {
        super::encoding::decode(head, encoding_name, &bytes)
    } else {
        Ok(Value::string(
            decode_string_complete_table(&bytes, OEM_DECODE[&dec_table_id]),
            head,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Decode)
    }
}
