use crate::{ast::RangeInclusion, ShellError, Value};

use indicatif::{HumanDuration, ProgressBar, ProgressState, ProgressStyle};
use std::fmt::Write;

pub fn get_progress_bar_from_value(value: &Value) -> Result<ProgressBar, ShellError> {
    let progress_bar_length = {
        match value {
            Value::List { vals, .. } => vals.len() as u64,
            Value::Range { val, .. } => {
                let from = get_value_as_i64(&val.from)?;
                let to = get_value_as_i64(&val.to)?;
                let inclusive_range = if val.inclusion == RangeInclusion::Inclusive {
                    1
                } else {
                    0
                };
                ((from - to).abs() as u64) + inclusive_range
            }
            _ => unreachable!(),
        }
    };
    get_progress_bar(progress_bar_length)
}

pub fn get_progress_bar(len: u64) -> Result<ProgressBar, ShellError> {
    let progress_bar = ProgressBar::new(len);
    let style = get_default_style();
    progress_bar.set_style(style);
    Ok(progress_bar)
}

fn get_default_style() -> ProgressStyle {
    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] ({eta})",
    )
    .expect("to be a valid template string.")
    .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
        write!(w, "{:.1}", HumanDuration(state.eta())).expect("To have a valid eta")
    })
    .progress_chars("#>-");
    style
}

fn get_value_as_i64(value: &Value) -> Result<i64, ShellError> {
    let mut val = 0;
    if let Ok(v) = value.as_i64() {
        val = v;
    } else if let Ok(v) = value.as_f64() {
        val = v.floor() as i64;
    }
    Ok(val)
}
