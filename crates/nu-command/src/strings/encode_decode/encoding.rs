use encoding_rs::Encoding;
use nu_protocol::{ShellError, Span, Spanned, Value};

pub fn decode(head: Span, encoding: Spanned<String>, bytes: &[u8]) -> Result<Value, ShellError> {
    let encoding = parse_encoding(encoding.span, &encoding.item)?;
    let (result, ..) = encoding.decode(bytes);
    Ok(Value::String {
        val: result.into_owned(),
        span: head,
    })
}

pub fn encode(head: Span, encoding: Spanned<String>, s: &str) -> Result<Value, ShellError> {
    let encoding = parse_encoding(encoding.span, &encoding.item)?;
    let (result, ..) = encoding.encode(s);
    Ok(Value::Binary {
        val: result.into_owned(),
        span: head,
    })
}

fn parse_encoding(span: Span, label: &str) -> Result<&'static Encoding, ShellError> {
    match Encoding::for_label_no_replacement(label.as_bytes()) {
        None => Err(ShellError::GenericError(
            format!(
                r#"{} is not a valid encoding, refer to https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics for a valid list of encodings"#,
                label
            ),
            "invalid encoding".into(),
            Some(span),
            None,
            Vec::new(),
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
    fn smoke(#[case] encoding: String, #[case] expected: &str) {
        let test_span = Span::test_data();
        let encoding = Spanned {
            item: encoding,
            span: test_span,
        };

        let encoded = encode(test_span, encoding.clone(), expected).unwrap();
        let encoded = encoded.as_binary().unwrap();

        let decoded = decode(test_span, encoding, encoded).unwrap();
        let decoded = decoded.as_string().unwrap();

        assert_eq!(decoded, expected);
    }
}
