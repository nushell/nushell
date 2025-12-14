//! Syntax highlighting for nushell code in the explore config TUI.
//!
//! This module provides syntax highlighting functionality that converts nushell code
//! into ANSI-styled text that can be rendered in the TUI.

use nu_ansi_term::Style;
use nu_color_config::get_shape_color;
use nu_parser::{flatten_block, parse};
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use std::sync::Arc;

/// Result of syntax highlighting multi-line nushell code
pub struct HighlightedContent {
    /// The ANSI-styled lines
    pub lines: Vec<String>,
}

/// Highlight a multi-line string of nushell code and return ANSI-styled lines.
///
/// This function parses the entire input as nushell code and applies syntax highlighting
/// colors based on the user's configuration. The result is split into lines to match
/// the original line structure.
///
/// By parsing the entire content at once, multi-line constructs like records, lists,
/// and closures are properly recognized and highlighted.
pub fn highlight_nushell_content(
    engine_state: &Arc<EngineState>,
    stack: &Arc<Stack>,
    content: &str,
) -> HighlightedContent {
    if content.is_empty() {
        return HighlightedContent {
            lines: vec![String::new()],
        };
    }

    let config = stack.get_config(engine_state);
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, content.as_bytes(), false);
    let shapes = flatten_block(&working_set, &block);
    let global_span_offset = engine_state.next_span_start();

    let mut result = String::new();
    let mut last_seen_span_end = global_span_offset;

    for (raw_span, flat_shape) in &shapes {
        // Handle alias spans for external commands
        let span = if let nu_parser::FlatShape::External(alias_span) = flat_shape {
            alias_span
        } else {
            raw_span
        };

        if span.end <= last_seen_span_end
            || last_seen_span_end < global_span_offset
            || span.start < global_span_offset
        {
            // We've already output something for this span, skip it
            continue;
        }

        // Add any gap between the last span and this one (e.g., whitespace, newlines)
        if span.start > last_seen_span_end {
            let gap = &content
                [(last_seen_span_end - global_span_offset)..(span.start - global_span_offset)];
            result.push_str(gap);
        }

        // Get the token text
        let token = &content[(span.start - global_span_offset)..(span.end - global_span_offset)];

        // Get the style for this shape and apply it
        let style = get_shape_color(flat_shape.as_str(), &config);
        result.push_str(&style.paint(token).to_string());

        last_seen_span_end = span.end;
    }

    // Add any remaining text after the last span
    let remainder = &content[(last_seen_span_end - global_span_offset)..];
    if !remainder.is_empty() {
        result.push_str(&Style::new().paint(remainder).to_string());
    }

    // Split the highlighted result into lines
    // We need to handle this carefully to preserve ANSI codes across line boundaries
    let lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();

    // Handle the edge case where content ends with a newline
    // (lines() doesn't include an empty string at the end for trailing newlines)
    let final_lines = if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    };

    HighlightedContent { lines: final_lines }
}
