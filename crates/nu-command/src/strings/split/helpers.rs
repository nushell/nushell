use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

#[derive(Clone, Copy)]
pub enum SplitWhere {
    On,
    Before,
    After,
}

impl FromValue for SplitWhere {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let s = <String>::from_value(v)?;
        match s.as_str() {
            "on" => Ok(SplitWhere::On),
            "before" => Ok(SplitWhere::Before),
            "after" => Ok(SplitWhere::After),
            _ => Err(ShellError::InvalidValue {
                valid: "one of: on, before, after".into(),
                actual: s,
                span,
            }),
        }
    }
}

pub fn split_str(
    s: &str,
    regex: &Regex,
    max_split: Option<usize>,
    collapse_empty: bool,
    split: SplitWhere,
    v_span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut res = vec![];
    let mut last_idx = 0;

    for capture in regex.captures_iter(s) {
        if let Some(max) = max_split {
            if res.len() + 1 == max {
                break;
            }
        }

        let capture = capture.map_err(|err| ShellError::GenericError {
            error: "Error with regular expression".into(),
            msg: err.to_string(),
            span: Some(v_span),
            help: None,
            inner: vec![],
        })?;

        let Some(m) = capture.get(0) else {
            return Err(ShellError::NushellFailed {
                msg: "capture.get(0) should always return the full regex match".to_string(),
            });
        };

        let s_part: &str;

        match split {
            SplitWhere::On => {
                s_part = &s[last_idx..m.start()];
                last_idx = m.end();
            }
            SplitWhere::Before => {
                s_part = &s[last_idx..m.start()];
                last_idx = m.start();
            }
            SplitWhere::After => {
                s_part = &s[last_idx..m.end()];
                last_idx = m.end();
            }
        }

        let v = s_part.into_value(v_span);

        if !collapse_empty || !v.is_empty() {
            res.push(v);
        }
    }

    let v = s[last_idx..s.len()].into_value(v_span);
    if !collapse_empty || !v.is_empty() {
        res.push(v);
    }

    Ok(res)
}
