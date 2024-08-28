#![allow(unused)]

use data_encoding::Encoding;

use nu_engine::command_prelude::*;

mod base32;
mod base32hex;
mod base64;
mod hex;

pub use base32::{DecodeBase32, EncodeBase32};
pub use base32hex::{DecodeBase32Hex, EncodeBase32Hex};
pub use base64::{DecodeBase64, EncodeBase64};
pub use hex::{DecodeHex, EncodeHex};

pub fn decode(
    encoding: Encoding,
    call_span: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    let (input_str, input_span) = get_string(input, call_span)?;
    let output = match encoding.decode(input_str.as_bytes()) {
        Ok(output) => output,
        Err(err) => {
            return Err(ShellError::IncorrectValue {
                msg: err.to_string(),
                val_span: input_span,
                call_span,
            });
        }
    };

    Ok(Value::binary(output, call_span).into_pipeline_data_with_metadata(metadata))
}

pub fn encode(
    encoding: Encoding,
    call_span: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    let (input_bytes, _) = get_binary(input, call_span)?;
    let output = encoding.encode(&input_bytes);

    Ok(Value::string(output, call_span).into_pipeline_data_with_metadata(metadata))
}

fn get_string(input: PipelineData, call_span: Span) -> Result<(String, Span), ShellError> {
    match input {
        PipelineData::Value(val, ..) => {
            let span = val.span();
            match val {
                Value::String { val, .. } => Ok((val, span)),

                value => Err(ShellError::TypeMismatch {
                    err_message: "binary or string".to_owned(),
                    span: call_span,
                }),
            }
        }
        PipelineData::ListStream(list, ..) => Err(ShellError::PipelineMismatch {
            exp_input_type: "binary or string".to_owned(),
            dst_span: call_span,
            src_span: list.span(),
        }),
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            Ok((stream.into_string()?, span))
        }
        PipelineData::Empty => Err(ShellError::PipelineEmpty {
            dst_span: call_span,
        }),
    }
}

fn get_binary(input: PipelineData, call_span: Span) -> Result<(Vec<u8>, Span), ShellError> {
    match input {
        PipelineData::Value(val, ..) => {
            let span = val.span();
            match val {
                Value::Binary { val, .. } => Ok((val, span)),
                Value::String { val, .. } => Ok((val.into_bytes(), span)),

                value => Err(ShellError::TypeMismatch {
                    err_message: "binary or string".to_owned(),
                    span: call_span,
                }),
            }
        }
        PipelineData::ListStream(list, ..) => Err(ShellError::PipelineMismatch {
            exp_input_type: "binary or string".to_owned(),
            dst_span: call_span,
            src_span: list.span(),
        }),
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            Ok((stream.into_bytes()?, span))
        }
        PipelineData::Empty => Err(ShellError::PipelineEmpty {
            dst_span: call_span,
        }),
    }
}
