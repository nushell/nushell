use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::value::StrExt;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::cmp;

pub struct Seq;

#[derive(Deserialize)]
pub struct SeqArgs {
    rest: Vec<Tagged<f64>>,
    separator: Option<Tagged<String>>,
    terminator: Option<Tagged<String>>,
    widths: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for Seq {
    fn name(&self) -> &str {
        "seq"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq")
            .rest(SyntaxShape::Number, "sequence values")
            .named(
                "separator",
                SyntaxShape::String,
                "separator character (defaults to \\n)",
                Some('s'),
            )
            .named(
                "terminator",
                SyntaxShape::String,
                "terminator character (defaults to \\n)",
                Some('t'),
            )
            .switch(
                "widths",
                "equalize widths of all numbers by padding with zeros",
                Some('w'),
            )
    }

    fn usage(&self) -> &str {
        "print sequences of numbers"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        seq(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence 1 to 10 with newline separator",
                example: "seq 1 10",
                result: Some(vec![
                    UntaggedValue::string("1").into(),
                    UntaggedValue::string("2").into(),
                    UntaggedValue::string("3").into(),
                    UntaggedValue::string("4").into(),
                    UntaggedValue::string("5").into(),
                    UntaggedValue::string("6").into(),
                    UntaggedValue::string("7").into(),
                    UntaggedValue::string("8").into(),
                    UntaggedValue::string("9").into(),
                    UntaggedValue::string("10").into(),
                ]),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator",
                example: "seq -s '|' 1 10",
                result: Some(vec![Value::from("1|2|3|4|5|6|7|8|9|10")]),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator padded with 0",
                example: "seq -s '|' -w 1 10",
                result: Some(vec![Value::from("01|02|03|04|05|06|07|08|09|10")]),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator padded by 2s",
                example: "seq -s ' | ' -w 1 2 10",
                result: Some(vec![Value::from("01 | 03 | 05 | 07 | 09")]),
            },
        ]
    }
}

async fn seq(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();

    let (
        SeqArgs {
            rest: rest_nums,
            separator,
            terminator,
            widths,
        },
        _,
    ) = args.process().await?;

    if rest_nums.is_empty() {
        return Err(ShellError::labeled_error(
            "seq requires some parameters",
            "needs parameter",
            name,
        ));
    }

    let sep: String = match separator {
        Some(s) => {
            if s.item == r"\t" {
                '\t'.to_string()
            } else if s.item == r"\n" {
                '\n'.to_string()
            } else if s.item == r"\r" {
                '\r'.to_string()
            } else {
                let vec_s: Vec<char> = s.chars().collect();
                if vec_s.is_empty() {
                    return Err(ShellError::labeled_error(
                        "Expected a single separator char from --separator",
                        "requires a single character string input",
                        &s.tag,
                    ));
                };
                vec_s.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let term: String = match terminator {
        Some(t) => {
            if t.item == r"\t" {
                '\t'.to_string()
            } else if t.item == r"\n" {
                '\n'.to_string()
            } else if t.item == r"\r" {
                '\r'.to_string()
            } else {
                let vec_t: Vec<char> = t.chars().collect();
                if vec_t.is_empty() {
                    return Err(ShellError::labeled_error(
                        "Expected a single terminator char from --terminator",
                        "requires a single character string input",
                        &t.tag,
                    ));
                };
                vec_t.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let rest_nums: Vec<String> = rest_nums.iter().map(|n| n.item.to_string()).collect();

    run_seq(sep, Some(term), widths.item, rest_nums)
}

#[cfg(test)]
mod tests {
    use super::Seq;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Seq {})
    }
}

fn parse_float(mut s: &str) -> Result<f64, String> {
    if s.starts_with('+') {
        s = &s[1..];
    }
    match s.parse() {
        Ok(n) => Ok(n),
        Err(e) => Err(format!(
            "seq: invalid floating point argument `{}`: {}",
            s, e
        )),
    }
}

fn escape_sequences(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\t", "\t")
}

pub fn run_seq(
    sep: String,
    termy: Option<String>,
    widths: bool,
    free: Vec<String>,
) -> Result<OutputStream, ShellError> {
    let mut largest_dec = 0;
    let mut padding = 0;
    let first = if free.len() > 1 {
        let slice = &free[0][..];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = len - dec;
        padding = dec;
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                return Err(ShellError::labeled_error(
                    s,
                    "error parsing float",
                    Tag::unknown(),
                ));
            }
        }
    } else {
        1.0
    };
    let step = if free.len() > 2 {
        let slice = &free[1][..];
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = cmp::max(largest_dec, len - dec);
        padding = cmp::max(padding, dec);
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                return Err(ShellError::labeled_error(
                    s,
                    "error parsing float",
                    Tag::unknown(),
                ));
            }
        }
    } else {
        1.0
    };
    let last = {
        let slice = &free[free.len() - 1][..];
        padding = cmp::max(padding, slice.find('.').unwrap_or_else(|| slice.len()));
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                return Err(ShellError::labeled_error(
                    s,
                    "error parsing float",
                    Tag::unknown(),
                ));
            }
        }
    };
    if largest_dec > 0 {
        largest_dec -= 1;
    }
    let separator = escape_sequences(&sep[..]);
    let terminator = match termy {
        Some(term) => escape_sequences(&term[..]),
        None => separator.clone(),
    };
    Ok(print_seq(
        first,
        step,
        last,
        largest_dec,
        separator,
        terminator,
        widths,
        padding,
    ))
}

fn done_printing(next: f64, step: f64, last: f64) -> bool {
    if step >= 0f64 {
        next > last
    } else {
        next < last
    }
}

#[allow(clippy::too_many_arguments)]
fn print_seq(
    first: f64,
    step: f64,
    last: f64,
    largest_dec: usize,
    separator: String,
    terminator: String,
    pad: bool,
    padding: usize,
) -> OutputStream {
    let mut i = 0isize;
    let mut value = first + i as f64 * step;
    let mut ret_str = "".to_owned();
    while !done_printing(value, step, last) {
        let istr = format!("{:.*}", largest_dec, value);
        let ilen = istr.len();
        let before_dec = istr.find('.').unwrap_or(ilen);
        if pad && before_dec < padding {
            for _ in 0..(padding - before_dec) {
                ret_str.push('0');
            }
        }
        ret_str.push_str(&istr);
        i += 1;
        value = first + i as f64 * step;
        if !done_printing(value, step, last) {
            ret_str.push_str(&separator);
        }
    }

    if (first >= last && step < 0f64) || (first <= last && step > 0f64) {
        ret_str.push_str(&terminator);
    }

    let rows: Vec<Value> = ret_str
        .lines()
        .map(|v| v.to_str_value_create_tag())
        .collect();
    futures::stream::iter(rows.into_iter().map(ReturnSuccess::value)).to_output_stream()
}
