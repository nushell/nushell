use log::trace;
use nu_ansi_term::Style;
use nu_color_config::get_shape_color;
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_protocol::Config;
use reedline::{Highlighter, StyledText};

pub struct NuHighlighter {
    pub engine_state: EngineState,
    pub config: Config,
}

impl Highlighter for NuHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        trace!("highlighting: {}", line);

        let (shapes, global_span_offset) = {
            let mut working_set = StateWorkingSet::new(&self.engine_state);
            let (block, _) = parse(&mut working_set, None, line.as_bytes(), false, &[]);

            let shapes = flatten_block(&working_set, &block);
            (shapes, self.engine_state.next_span_start())
        };

        let mut output = StyledText::default();
        let mut last_seen_span = global_span_offset;

        for shape in &shapes {
            if shape.0.end <= last_seen_span
                || last_seen_span < global_span_offset
                || shape.0.start < global_span_offset
            {
                // We've already output something for this span
                // so just skip this one
                continue;
            }
            if shape.0.start > last_seen_span {
                let gap = line
                    [(last_seen_span - global_span_offset)..(shape.0.start - global_span_offset)]
                    .to_string();
                output.push((Style::new(), gap));
            }
            let next_token = line
                [(shape.0.start - global_span_offset)..(shape.0.end - global_span_offset)]
                .to_string();
            match shape.1 {
                FlatShape::Garbage => output.push((
                    // nushell Garbage
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Nothing => output.push((
                    // nushell Nothing
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Binary => {
                    // nushell ?
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Bool => {
                    // nushell ?
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Int => {
                    // nushell Int
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Float => {
                    // nushell Decimal
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Range => output.push((
                    // nushell DotDot ?
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::InternalCall => output.push((
                    // nushell InternalCommand
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::External => {
                    // nushell ExternalCommand
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::ExternalArg => {
                    // nushell ExternalWord
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Literal => {
                    // nushell ?
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Operator => output.push((
                    // nushell Operator
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Signature => output.push((
                    // nushell ?
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::String => {
                    // nushell String
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::StringInterpolation => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::DateTime => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::List => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Table => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Record => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Block => {
                    // nushell ???
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Filepath => output.push((
                    // nushell Path
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Directory => output.push((
                    // nushell Directory
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::GlobPattern => output.push((
                    // nushell GlobPattern
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Variable => output.push((
                    // nushell Variable
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
                FlatShape::Flag => {
                    // nushell Flag
                    output.push((
                        get_shape_color(shape.1.to_string(), &self.config),
                        next_token,
                    ))
                }
                FlatShape::Custom(..) => output.push((
                    get_shape_color(shape.1.to_string(), &self.config),
                    next_token,
                )),
            }
            last_seen_span = shape.0.end;
        }

        let remainder = line[(last_seen_span - global_span_offset)..].to_string();
        if !remainder.is_empty() {
            output.push((Style::new(), remainder));
        }

        output
    }
}
