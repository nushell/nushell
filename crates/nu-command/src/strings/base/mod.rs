use nu_protocol::{PipelineData, ShellError, Span, Value};

mod decode;
mod encode;

pub use decode::DecodeBase;
pub use encode::EncodeBase;

pub fn encoding(
    name: &str,
    val_span: Span,
    call_span: Span,
) -> Result<data_encoding::Encoding, ShellError> {
    match name {
        "base32" => Ok(data_encoding::BASE32),
        "base32hex" => Ok(data_encoding::BASE32HEX),
        "base32hex_nopad" => Ok(data_encoding::BASE32HEX_NOPAD),
        "base32_dnscurve" => Ok(data_encoding::BASE32_DNSCURVE),
        "base32_dnssec" => Ok(data_encoding::BASE32_DNSSEC),
        "base32_nopad" => Ok(data_encoding::BASE32_NOPAD),
        "base64" => Ok(data_encoding::BASE64),
        "base64url" => Ok(data_encoding::BASE64URL),
        "base64url_nopad" => Ok(data_encoding::BASE64URL_NOPAD),
        "base64_mime" => Ok(data_encoding::BASE64_MIME),
        "base64_mime_permissive" => Ok(data_encoding::BASE64_MIME_PERMISSIVE),
        "base64_nopad" => Ok(data_encoding::BASE64_NOPAD),
        "hexlower" => Ok(data_encoding::HEXLOWER),
        "hexlower_permissive" => Ok(data_encoding::HEXLOWER_PERMISSIVE),
        "hexupper" => Ok(data_encoding::HEXUPPER),
        "hexupper_permissive" => Ok(data_encoding::HEXUPPER_PERMISSIVE),
        _ => Err(ShellError::IncorrectValue {
            msg: format!("Encoding '{name}' not found"),
            val_span,
            call_span,
        }),
    }
}

pub fn get_string(input: PipelineData, call_span: Span) -> Result<(String, Span), ShellError> {
    match input {
        PipelineData::Value(val, ..) => {
            let span = val.span();
            match val {
                Value::String { val, .. } => Ok((val, span)),

                _ => {
                    todo!("Invalid type")
                }
            }
        }
        PipelineData::ListStream(..) => {
            todo!()
        }
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            Ok((stream.into_string()?, span))
        }
        PipelineData::Empty => {
            todo!("Can't have empty data");
        }
    }
}

pub fn get_binary(input: PipelineData, call_span: Span) -> Result<(Vec<u8>, Span), ShellError> {
    match input {
        PipelineData::Value(val, ..) => {
            let span = val.span();
            match val {
                Value::Binary { val, .. } => Ok((val, span)),
		Value::String { val, .. } => Ok((val.into_bytes(), span)),

                _ => {
                    todo!("Invalid type")
                }
            }
        }
        PipelineData::ListStream(..) => {
            todo!()
        }
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            Ok((stream.into_bytes()?, span))
        }
        PipelineData::Empty => {
            todo!("Can't have empty data");
        }
    }
}
