use nu_protocol::{ShellError, Span};

pub fn chain_error_with_input(
    error_source: ShellError,
    input_span: Result<Span, ShellError>,
) -> ShellError {
    if let Ok(span) = input_span {
        return ShellError::EvalBlockWithInput(span, vec![error_source]);
    }
    error_source
}
