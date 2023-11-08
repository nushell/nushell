use log::trace;
use nu_ansi_term::Style;
use nu_color_config::get_shape_color;
use nu_parser::{flatten_block, parse};
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_protocol::Config;
use reedline::{Highlighter, StyledText};
use std::sync::Arc;

pub struct NuHighlighter {
    pub engine_state: Arc<EngineState>,
    pub config: Config,
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        trace!("highlighting: {}", line);

        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = parse(&mut working_set, None, line.as_bytes(), false);

        let shapes = flatten_block(&working_set, &block);
        let global_span_offset = self.engine_state.next_span_start();

        let mut output = StyledText::default();
        let mut last_seen_span = 0;

        for shape in &shapes {
            // shape span start/end with the offset
            let start = shape.0.start - global_span_offset;
            let end = shape.0.end - global_span_offset;

            // fill the gap between tokens (if present) with default style
            if start > last_seen_span {
                let gap = line[last_seen_span..start].to_string();
                output.push((Style::new(), gap));
            }

            let token = line[start..end].to_string();
            let token_style = get_shape_color(shape.1.to_string(), &self.config);
            output.push((token_style, token));

            last_seen_span = end;
        }

        let remainder = line[last_seen_span..].to_string();
        output.push((Style::new(), remainder));

        output
    }
}
