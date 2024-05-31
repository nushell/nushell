use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use nu_protocol::{ShellError, Span, Spanned, Value};

pub fn detect_encoding_name(
    head: Span,
    input: Span,
    bytes: &[u8],
) -> Result<&'static Encoding, ShellError> {
    let mut detector = EncodingDetector::new();
    let _non_ascii = detector.feed(bytes, false);
    //Guess(TLD=None(usually used in HTML), Allow_UTF8=True)
    let (encoding, is_certain) = detector.guess_assess(None, true);
    if !is_certain {
        return Err(ShellError::UnsupportedInput {
            msg: "Input contains unknown encoding, try giving a encoding name".into(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: input,
        });
    }
    Ok(encoding)
}

pub fn decode(
    head: Span,
    encoding_name: Spanned<String>,
    bytes: &[u8],
) -> Result<Value, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let encoding = if encoding_name.item.eq_ignore_ascii_case("utf16") {
        parse_encoding(encoding_name.span, "utf-16")
    } else {
        parse_encoding(encoding_name.span, &encoding_name.item)
    }?;
    let (result, ..) = encoding.decode(bytes);
    Ok(Value::string(result.into_owned(), head))
}

pub fn encode(
    head: Span,
    encoding_name: Spanned<String>,
    s: &str,
    s_span: Span,
    ignore_errors: bool,
) -> Result<Value, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let encoding = if encoding_name.item.eq_ignore_ascii_case("utf16") {
        parse_encoding(encoding_name.span, "utf-16")
    } else {
        parse_encoding(encoding_name.span, &encoding_name.item)
    }?;

    // Since the Encoding Standard doesn't specify encoders for "UTF-16BE" and "UTF-16LE"
    // Check if the encoding is one of them and return an error
    if ["UTF-16BE", "UTF-16LE"].contains(&encoding.name()) {
        return Err(ShellError::GenericError {
            error: format!(r#"{} encoding is not supported"#, &encoding_name.item),
            msg: "invalid encoding".into(),
            span: Some(encoding_name.span),
            help: Some("refer to https://docs.rs/encoding_rs/latest/encoding_rs/index.html#statics for a valid list of encodings".into()),
            inner: vec![],
        });
    }

    let (result, _actual_encoding, replacements) = encoding.encode(s);
    // Because encoding_rs is a Web-facing crate, it defaults to replacing unknowns with HTML entities.
    // This behaviour can be enabled with -i. Otherwise, it becomes an error.
    if replacements && !ignore_errors {
        // TODO: make GenericError accept two spans (including head)
        Err(ShellError::GenericError {
            error: "error while encoding string".into(),
            msg: format!("string contained characters not in {}", &encoding_name.item),
            span: Some(s_span),
            help: None,
            inner: vec![],
        })
    } else {
        Ok(Value::binary(result.into_owned(), head))
    }
}

fn parse_encoding(span: Span, label: &str) -> Result<&'static Encoding, ShellError> {
    // Workaround for a bug in the Encodings Specification.
    let label = if label.eq_ignore_ascii_case("utf16") {
        "utf-16"
    } else {
        label
    };
    match Encoding::for_label_no_replacement(label.as_bytes()) {
        None => Err(ShellError::GenericError{
            error: format!(
                r#"{label} is not a valid encoding"#
            ),
            msg: "invalid encoding".into(),
            span: Some(span),
            help: Some("refer to https://docs.rs/encoding_rs/latest/encoding_rs/index.html#statics for a valid list of encodings".into()),
            inner: vec![],
        }),
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
    // Tests for specific renditions of UTF-8 labels
    #[case::utf8("utf8", "")]
    #[case::utf_hyphen_8("utf-8", "")]
    fn smoke(#[case] encoding: String, #[case] expected: &str) {
        let test_span = Span::test_data();
        let encoding = Spanned {
            item: encoding,
            span: test_span,
        };

        let encoded = encode(test_span, encoding.clone(), expected, test_span, true).unwrap();
        let encoded = encoded.coerce_into_binary().unwrap();

        let decoded = decode(test_span, encoding, &encoded).unwrap();
        let decoded = decoded.coerce_into_string().unwrap();

        assert_eq!(decoded, expected);
    }

    #[rstest]
    #[case::big5(&[186, 251, 176, 242, 164, 106, 168, 229, 161, 93, 87, 105, 107, 105, 112, 101, 100, 105, 97, 161,
        94, 170, 204, 161, 65, 186, 244, 184, 244, 172, 176, 194, 166, 161, 70, 182, 176, 164, 209, 164, 85,
         170, 190, 161, 66, 165, 124, 174, 252, 168, 165, 161, 66, 178, 179, 164, 72, 167, 211, 161, 65, 174,
         209, 166, 202, 172, 236, 178, 106, 161, 67, 169, 108, 167, 64, 170, 204, 161, 65, 186, 251, 176,
         242, 180, 67, 197, 233, 176, 242, 170, 247, 183, 124, 164, 93, 161, 67], "Big5")]
    // FIXME: chardetng fails on this
    //#[case::shiftjis(&[130, 162, 130, 235, 130, 205, 130, 201, 130, 217, 130, 214, 130, 198, 129, 64, 130,
    //    191, 130, 232, 130, 202, 130, 233, 130, 240], "SHIFT_JIS")]
    #[case::eucjp(&[164, 164, 164, 237, 164, 207, 164, 203, 164, 219, 164, 216, 164, 200, 161, 161, 164, 193,
        164, 234, 164, 204, 164, 235, 164, 242],"EUC-JP")]
    #[case::euckr(&[192, 167, 197, 176, 185, 233, 176, 250, 40, 45, 219, 221, 206, 161, 41, 32, 182, 199, 180,
         194, 32, 192, 167, 197, 176, 199, 199, 181, 240, 190, 198, 180, 194, 32, 180, 169, 177, 184, 179, 170,
          32, 192, 218, 192, 175, 183, 211, 176, 212, 32, 190, 181, 32, 188, 246, 32, 192, 214, 180, 194, 32, 180,
           217, 190, 240, 190, 238, 198, 199, 32, 192, 206, 197, 205, 179, 221, 32, 185, 233, 176, 250, 187, 231, 192,
            252, 192, 204, 180, 217],"EUC-KR")]
    #[case::gb2312(&[206, 172, 187, 249, 202, 199, 210, 187, 214, 214, 212, 218, 205, 248, 194, 231, 201, 207, 191,
         170, 183, 197, 199, 210, 191, 201, 185, 169, 182, 224, 200, 203, 208, 173, 205, 172, 180, 180, 215, 247,
          181, 196, 179, 172, 206, 196, 177, 190, 207, 181, 205, 179, 163, 172, 211, 201, 195, 192, 185, 250, 200,
           203, 206, 214, 181, 194, 161, 164, 191, 178, 196, 254, 176, 178, 211, 218, 49, 57, 57, 53, 196, 234, 202,
            215, 207, 200, 191, 170, 183, 162], "GB2312")]
    #[case::tis620(&[199, 212, 185, 226, 180, 199, 202, 236, 45, 49, 50, 53, 50, 224, 187, 231, 185, 195, 203, 209,
         202, 205, 209, 161, 162, 195, 208, 225, 186, 186, 203, 185, 214, 232, 167, 228, 186, 181, 236, 183, 213,
          232, 227, 170, 233, 161, 209, 186, 205, 209, 161, 201, 195, 197, 208, 181, 212, 185, 32, 193, 209, 161,
           182, 217, 161, 227, 170, 233, 227, 185, 205, 167, 164, 236, 187, 195, 208, 161, 205, 186, 195, 216, 232,
            185, 224, 161, 232, 210, 227, 185, 228, 193, 226, 164, 195, 171, 205, 191, 183, 236], "TIS-620")]
    fn smoke_encoding_name(#[case] bytes: &[u8], #[case] expected: &str) {
        let encoding_name =
            detect_encoding_name(Span::test_data(), Span::test_data(), bytes).unwrap();
        assert_eq!(
            encoding_name,
            Encoding::for_label(expected.as_bytes()).unwrap()
        );
    }
}
