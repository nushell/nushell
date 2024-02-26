use crate::ast::PipelineElement;
use crate::debugger::Debugger;
use crate::engine::EngineState;
use crate::record;
use crate::{PipelineData, ShellError, Span, Value};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
struct ProfilerInfo {
    depth: i64,
    element_span: Span,
    element_input: Option<Value>,
}

/// Basic profiler
#[derive(Debug, Clone)]
pub struct Profiler {
    depth: i64,
    max_depth: i64,
    source_fragments: HashMap<(usize, usize), String>,
    element_start_times: Vec<Instant>,
    element_durations_sec: Vec<(ProfilerInfo, f64)>,
    collect_spans: bool,
    collect_source: bool,
    collect_expanded_source: bool,
    collect_values: bool,
}

impl Profiler {
    pub fn new(
        max_depth: i64,
        collect_spans: bool,
        collect_source: bool,
        collect_expanded_source: bool,
        collect_values: bool,
    ) -> Self {
        Profiler {
            depth: 0,
            max_depth,
            source_fragments: HashMap::new(),
            element_start_times: vec![],
            element_durations_sec: vec![],
            collect_spans,
            collect_source,
            collect_expanded_source,
            collect_values,
        }
    }
}

impl Debugger for Profiler {
    fn enter_block(&mut self) {
        self.depth += 1;
    }

    fn leave_block(&mut self) {
        self.depth -= 1;
    }

    fn enter_element(&mut self) {
        if self.depth > self.max_depth {
            return;
        }

        self.element_start_times.push(Instant::now());
    }

    fn leave_element(
        &mut self,
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
        if self.depth > self.max_depth {
            return;
        }

        let Some(start) = self.element_start_times.pop() else {
            // TODO: Log internal errors
            eprintln!(
                "Error: Profiler left pipeline element without matching element start time stamp."
            );
            return;
        };

        let element_span = element.span();

        if self.collect_source {
            let source_fragment =
                String::from_utf8_lossy(engine_state.get_span_contents(element_span)).to_string();
            self.source_fragments
                .insert((element_span.start, element_span.end), source_fragment);
        }

        let inp_opt = if self.collect_values {
            Some(match input {
                Ok((pipeline_data, _not_sure_what_this_is)) => match pipeline_data {
                    PipelineData::Value(val, ..) => val.clone(),
                    PipelineData::ListStream(..) => Value::string("list stream", element_span),
                    PipelineData::ExternalStream { .. } => {
                        Value::string("external stream", element_span)
                    }
                    _ => Value::nothing(element_span),
                },
                Err(e) => Value::error(e.clone(), element_span),
            })
        } else {
            None
        };

        let info = ProfilerInfo {
            depth: self.depth,
            element_span,
            element_input: inp_opt,
        };

        self.element_durations_sec
            .push((info, start.elapsed().as_secs_f64()));
    }

    fn report(&self, profiler_span: Span) -> Result<Value, ShellError> {
        let mut rows = vec![];

        for (info, duration_sec) in self.element_durations_sec.iter() {
            let mut row = record! {
                "depth" => Value::int(info.depth, profiler_span)
            };

            if self.collect_spans {
                let span_start = i64::try_from(info.element_span.start).map_err(|_| {
                    profiler_error("error converting span start to i64", profiler_span)
                })?;
                let span_end = i64::try_from(info.element_span.end).map_err(|_| {
                    profiler_error("error converting span end to i64", profiler_span)
                })?;

                row.push(
                    "span",
                    Value::record(
                        record! {
                            "start" => Value::int(span_start, profiler_span),
                            "end" => Value::int(span_end, profiler_span),
                        },
                        profiler_span,
                    ),
                );
            }

            if self.collect_source {
                let Some(val) = self
                    .source_fragments
                    .get(&(info.element_span.start, info.element_span.end))
                else {
                    return Err(profiler_error(
                        "could not get source fragment",
                        profiler_span,
                    ));
                };

                let val = val.trim();
                let nlines = val.lines().count();

                let fragment = if self.collect_expanded_source {
                    val.to_string()
                } else {
                    let mut first_line = val.lines().next().unwrap_or("").to_string();

                    if nlines > 1 {
                        first_line.push_str(" ...");
                    }

                    first_line
                };

                row.push("source", Value::string(fragment, profiler_span));
            }

            if let Some(val) = &info.element_input {
                row.push("output", val.clone());
            }

            row.push(
                "duration_us",
                Value::float(duration_sec * 1e6, profiler_span),
            );

            rows.push(Value::record(row, profiler_span))
        }

        Ok(Value::list(rows, profiler_span))
    }
}

fn profiler_error(msg: impl Into<String>, span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Profiler Error".to_string(),
        msg: msg.into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}
