use crate::did_you_mean;
use nu_errors::ShellError;
use nu_protocol::Value;
use nu_source::Tagged;

pub fn suggestions(tried: Tagged<&str>, for_value: &Value) -> ShellError {
    let possibilities = did_you_mean(for_value, tried.to_string());

    match possibilities {
        Some(p) => ShellError::labeled_error(
            "Unknown column",
            format!("did you mean '{}'?", p[0]),
            tried.tag(),
        ),
        None => ShellError::labeled_error(
            "Unknown column",
            "row does not contain this column",
            tried.tag(),
        ),
    }
}
