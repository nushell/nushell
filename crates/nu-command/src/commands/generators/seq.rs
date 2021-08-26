use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::value::{DecimalExt, I64Ext, StrExt};
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::cmp;

pub struct Seq;

impl WholeStreamCommand for Seq {
    fn name(&self) -> &str {
        "seq"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq")
            .rest("rest", SyntaxShape::Number, "sequence values")
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
        "Print sequences of numbers."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        seq(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence 1 to 10 with newline separator",
                example: "seq 1 10",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(5).into(),
                    UntaggedValue::int(6).into(),
                    UntaggedValue::int(7).into(),
                    UntaggedValue::int(8).into(),
                    UntaggedValue::int(9).into(),
                    UntaggedValue::int(10).into(),
                ]),
            },
            Example {
                description: "sequence 1.0 to 2.0 by 0.1s with newline separator",
                example: "seq 1.0 0.1 2.0",
                result: Some(vec![
                    UntaggedValue::decimal_from_float(1.0000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.1000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.2000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.3000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.4000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.5000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.6000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.7000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.8000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(1.9000, Span::default()).into(),
                    UntaggedValue::decimal_from_float(2.0000, Span::default()).into(),
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

fn seq(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();

    let rest_nums: Vec<Tagged<f64>> = args.rest(0)?;
    let separator: Option<Tagged<String>> = args.get_flag("separator")?;
    let terminator: Option<Tagged<String>> = args.get_flag("terminator")?;
    let widths = args.has_flag("widths");

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

    run_seq(sep, Some(term), widths, rest_nums)
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
    // for string output
    let mut ret_str = "".to_owned();
    // for number output
    let mut ret_num = vec![];
    // If the separator and terminator are line endings we can convert to numbers
    let use_num =
        (separator == "\n" || separator == "\r") && (terminator == "\n" || terminator == "\r");

    while !done_printing(value, step, last) {
        if use_num {
            ret_num.push(value);
        } else {
            // formatting for string output with potential padding
            let istr = format!("{:.*}", largest_dec, value);
            let ilen = istr.len();
            let before_dec = istr.find('.').unwrap_or(ilen);
            if pad && before_dec < padding {
                for _ in 0..(padding - before_dec) {
                    ret_str.push('0');
                }
            }
            ret_str.push_str(&istr);
        }
        i += 1;
        value = first + i as f64 * step;
        if !done_printing(value, step, last) {
            ret_str.push_str(&separator);
        }
    }

    if !use_num && ((first >= last && step < 0f64) || (first <= last && step > 0f64)) {
        ret_str.push_str(&terminator);
    }

    if use_num {
        // we'd like to keep the datatype the same for the output, so check
        // and see if any of the output is really decimals, and if it is
        // we'll make the entire output decimals
        let contains_decimals = vec_contains_decimals(&ret_num);
        let rows: Vec<Value> = ret_num
            .iter()
            .map(|v| {
                if contains_decimals {
                    v.to_value_create_tag()
                } else {
                    let vi64 = *v as i64;
                    vi64.to_value_create_tag()
                }
            })
            .collect();
        (rows.into_iter()).into_output_stream()
    } else {
        let rows: Vec<Value> = ret_str
            .lines()
            .map(|v| v.to_str_value_create_tag())
            .collect();
        (rows.into_iter()).into_output_stream()
    }
}

fn vec_contains_decimals(array: &[f64]) -> bool {
    let mut found_decimal = false;
    for x in array {
        if x.fract() != 0.0 {
            found_decimal = true;
            break;
        }
    }

    found_decimal
}
