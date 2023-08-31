use nu_protocol::{
    engine::EngineState,
    Span,
};

// Format string with source file: line_number content_of_span.
// If span contains newlines, then print sourcefile on line by itself,
// where it will look like a visual header
// and let the content (presumably indented code) print near er the left margin.
pub fn dbg_span_string(engine_state: &EngineState, span: &Span) -> String {
    let filename = if let Some(f) = engine_state.get_file_for_span(*span) {
        f
    } else {
        "unknown"
    };

    let content = String::from_utf8_lossy(engine_state.get_span_contents(*span)).to_string();

    format!(
        "{}:{:<5}{} {}",
        filename,
        engine_state.get_line_number(*span),
        (if content.contains('\n') { "\n" } else { "" }),
        &content
    )
}

// there should eventually be debug display routines for calls
// (showing evaluated inputs, outputs and arguments), maybe for other kinds of syntax entities.
