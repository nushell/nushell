use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, ListStream, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};
use std::cmp;

#[derive(Clone)]
pub struct Seq;

impl Command for Seq {
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
            .category(Category::Generators)
    }

    fn usage(&self) -> &str {
        "Print sequences of numbers."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        seq(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence 1 to 10 with newline separator",
                example: "seq 1 10",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                        Value::test_int(5),
                        Value::test_int(6),
                        Value::test_int(7),
                        Value::test_int(8),
                        Value::test_int(9),
                        Value::test_int(10),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "sequence 1.0 to 2.0 by 0.1s with newline separator",
                example: "seq 1.0 0.1 2.0",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_float(1.0000),
                        Value::test_float(1.1000),
                        Value::test_float(1.2000),
                        Value::test_float(1.3000),
                        Value::test_float(1.4000),
                        Value::test_float(1.5000),
                        Value::test_float(1.6000),
                        Value::test_float(1.7000),
                        Value::test_float(1.8000),
                        Value::test_float(1.9000),
                        Value::test_float(2.0000),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator",
                example: "seq -s '|' 1 10",
                result: Some(Value::test_string("1|2|3|4|5|6|7|8|9|10")),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator padded with 0",
                example: "seq -s '|' -w 1 10",
                result: Some(Value::test_string("01|02|03|04|05|06|07|08|09|10")),
            },
            Example {
                description: "sequence 1 to 10 with pipe separator padded by 2s",
                example: "seq -s ' | ' -w 1 2 10",
                result: Some(Value::test_string("01 | 03 | 05 | 07 | 09")),
            },
        ]
    }
}

fn seq(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let rest_vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let separator: Option<Spanned<String>> = call.get_flag(engine_state, stack, "separator")?;
    let terminator: Option<Spanned<String>> = call.get_flag(engine_state, stack, "terminator")?;
    let widths = call.has_flag("widths");

    let mut has_float = false;
    let mut rest_nums: Vec<Spanned<f64>> = Vec::new();
    for val in rest_vals {
        if let Value::Float { .. } = val {
            has_float = true;
        }
        match val.as_f64() {
            Ok(float) => rest_nums.push(Spanned {
                item: float,
                span: val.span().unwrap_or_else(|_| Span::unknown()),
            }),
            Err(_) => match val.as_i64() {
            Ok(int) => rest_nums.push(Spanned {
                item: int as f64,
                span: val.span().unwrap_or_else(|_| Span::unknown()),
            }),
            Err(e) => return Err(e),
            }
        }
    }

    if rest_nums.is_empty() {
        return Err(ShellError::GenericError(
            "seq requires some parameters".into(),
            "needs parameter".into(),
            Some(call.head),
            None,
            Vec::new(),
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
                let vec_s: Vec<char> = s.item.chars().collect();
                if vec_s.is_empty() {
                    return Err(ShellError::GenericError(
                        "Expected a single separator char from --separator".into(),
                        "requires a single character string input".into(),
                        Some(s.span),
                        None,
                        Vec::new(),
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
                let vec_t: Vec<char> = t.item.chars().collect();
                if vec_t.is_empty() {
                    return Err(ShellError::GenericError(
                        "Expected a single terminator char from --terminator".into(),
                        "requires a single character string input".into(),
                        Some(t.span),
                        None,
                        Vec::new(),
                    ));
                };
                vec_t.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let rest_nums: Vec<String> = rest_nums.iter().map(|n| n.item.to_string()).collect();

    run_seq(
        sep,
        Some(term),
        widths,
        rest_nums,
        span,
        has_float,
        engine_state,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

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

fn run_seq(
    sep: String,
    termy: Option<String>,
    widths: bool,
    free: Vec<String>,
    span: Span,
    has_float: bool,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
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
                return Err(ShellError::GenericError(
                    s,
                    "".to_string(),
                    None,
                    Some("error parsing float".into()),
                    Vec::new(),
                ))
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
                return Err(ShellError::GenericError(
                    s,
                    "".to_string(),
                    None,
                    Some("error parsing float".into()),
                    Vec::new(),
                ))
            }
        }
    } else {
        1.0
    };
    let last = {
        let slice = &free[free.len() - 1][..];
        padding = cmp::max(padding, slice.find('.').unwrap_or(slice.len()));
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => {
                return Err(ShellError::GenericError(
                    s,
                    "".to_string(),
                    None,
                    Some("error parsing float".into()),
                    Vec::new(),
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
        span,
        has_float,
        engine_state,
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
    span: Span,
    is_float_sequence: bool,
    engine_state: &EngineState,
) -> PipelineData {
    let mut i = 0isize;
    let mut value = first + i as f64 * step;
    // for string output
    let mut ret_str = "".to_owned();
    // If the separator and terminator are line endings we can convert to numbers
    let use_num =
        (separator == "\n" || separator == "\r") && (terminator == "\n" || terminator == "\r");

    if use_num {
        if is_float_sequence {
            return PipelineData::ListStream(
                ListStream {
                    stream: Box::new(Counter {
                        count: first,
                        step,
                        last,
                        index: 0,
                        span,
                    }),
                    ctrlc: engine_state.ctrlc.clone(),
                },
                None,
            );
        } else {
            return PipelineData::ListStream(
                ListStream {
                    stream: Box::new(Counter {
                        count: first as i64,
                        step: step as i64,
                        last: last as i64,
                        index: 0,
                        span,
                    }),
                    ctrlc: engine_state.ctrlc.clone(),
                },
                None,
            );
        }
    }

    while !done_printing(value, step, last) {
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
        i += 1;
        value = first + i as f64 * step;
        if !done_printing(value, step, last) && !use_num {
            ret_str.push_str(&separator);
        }
    }

    if (first >= last && step < 0f64) || (first <= last && step > 0f64) {
        ret_str.push_str(&terminator);
    }

    let rows: String = ret_str.lines().collect();
    Value::string(rows, span).into_pipeline_data()
}

struct Counter<T> {
    count: T,
    step: T,
    last: T,
    index: isize,
    span: Span,
}

impl Iterator for Counter<i64> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        if (self.count > self.last && self.step >= 0) || (self.count < self.last && self.step <= 0)
        {
            return None;
        }
        let ret = Some(Value::Int {
            val: self.count as i64,
            span: self.span,
        });
        self.count += self.step;
        ret
    }
}

impl Iterator for Counter<f64> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        let count = self.count + self.index as f64 * self.step;
        if (count > self.last && self.step >= 0.0) || (count < self.last && self.step <= 0.0) {
            return None;
        }
        self.index += 1;
        Some(Value::Float {
            val: count,
            span: self.span,
        })
    }
}
