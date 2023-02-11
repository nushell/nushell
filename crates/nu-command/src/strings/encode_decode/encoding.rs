use encoding_rs::Encoding;
use nu_protocol::{ShellError, Span, Spanned, Value};

pub fn decode(
    head: Span,
    encoding_name: Spanned<String>,
    bytes: &[u8],
) -> Result<Value, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let encoding = if encoding_name.item.to_lowercase() == "utf16" {
        parse_encoding(encoding_name.span, "utf-16")
    } else {
        parse_encoding(encoding_name.span, &encoding_name.item)
    }?;
    let (result, ..) = encoding.decode(bytes);
    Ok(Value::String {
        val: result.into_owned(),
        span: head,
    })
}

pub fn encode(
    head: Span,
    encoding_name: Spanned<String>,
    s: &str,
    s_span: Span,
    ignore_errors: bool,
) -> Result<Value, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let encoding = if encoding_name.item.to_lowercase() == "utf16" {
        parse_encoding(encoding_name.span, "utf-16")
    } else {
        parse_encoding(encoding_name.span, &encoding_name.item)
    }?;
    let (result, _actual_encoding, replacements) = encoding.encode(s);
    // Because encoding_rs is a Web-facing crate, it defaults to replacing unknowns with HTML entities.
    // This behaviour can be enabled with -i. Otherwise, it becomes an error.
    if replacements && !ignore_errors {
        // TODO: make GenericError accept two spans (including head)
        Err(ShellError::GenericError(
            "error while encoding string".into(),
            format!("string contained characters not in {}", &encoding_name.item),
            Some(s_span),
            None,
            vec![],
        ))
    } else {
        Ok(Value::Binary {
            val: result.into_owned(),
            span: head,
        })
    }
}

fn parse_encoding(span: Span, label: &str) -> Result<&'static Encoding, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let label = if label.to_lowercase() == "utf16" {
        "utf-16"
    } else {
        label
    };
    match Encoding::for_label_no_replacement(label.as_bytes()) {
        None => Err(ShellError::GenericError(
            format!(
                r#"{label} is not a valid encoding"#
            ),
            "invalid encoding".into(),
            Some(span),
            Some("refer to https://docs.rs/encoding_rs/latest/encoding_rs/index.html#statics for a valid list of encodings".into()),
            vec![],
        )),
        Some(encoding) => Ok(encoding),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::big5("big5", "簡体字")]
    #[case::shift_jis("shift-jis", "何だと？……無駄な努力だ？……百も承知だ！")]
    #[case::euc_jp("euc-jp", "だがな、勝つ望みがある時ばかり、戦うのとは訳が違うぞ！")]
    #[case::euc_kr("euc-kr", "가셨어요?")]
    #[case::gbk("gbk", "簡体字")]
    #[case::iso_8859_1("iso-8859-1", "Some ¼½¿ Data µ¶·¸¹º")]
    #[case::cp1252("cp1252", "Some ¼½¿ Data")]
    #[case::latin5("latin5", "Some ¼½¿ Data µ¶·¸¹º")]
    // Tests for specific renditions of UTF-16 and UTF-8 labels
    #[case::utf16("utf16", "")]
    #[case::utf_hyphen_16("utf-16", "")]
    #[case::utf8("utf8", "")]
    #[case::utf_hyphen_8("utf-8", "")]
    fn smoke(#[case] encoding: String, #[case] expected: &str) {
        let test_span = Span::test_data();
        let encoding = Spanned {
            item: encoding,
            span: test_span,
        };

        let encoded = encode(test_span, encoding.clone(), expected, test_span, true).unwrap();
        let encoded = encoded.as_binary().unwrap();

        let decoded = decode(test_span, encoding, encoded).unwrap();
        let decoded = decoded.as_string().unwrap();

        assert_eq!(decoded, expected);
    }
}
