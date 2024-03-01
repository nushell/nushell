use crate::ast::{Block, Expr, PipelineElement};
use crate::debugger::Debugger;
use crate::engine::EngineState;
use crate::record;
use crate::{PipelineData, ShellError, Span, Value};
use indexmap::IndexSet;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct ElementId(usize);

#[derive(Debug, Clone)]
struct ProfilerInfo {
    depth: i64,
    element_span: Span,
    element_input: Option<Value>,
    expr: Option<String>,
}

#[derive(Debug, Clone)]
struct ProfilerInfo2 {
    start: Instant,
    duration_sec: f64,
    depth: i64,
    element_span: Span,
    element_output: Option<Value>,
    expr: Option<String>,
    children: Vec<ElementId>,
}

impl ProfilerInfo2 {
    pub fn new(depth: i64, element_span: Span) -> Self {
        ProfilerInfo2 {
            start: Instant::now(),
            duration_sec: 0.0,
            depth,
            element_span,
            element_output: None,
            expr: None,
            children: vec![],
        }
    }

    // pub fn new(duration_sec: f64, depth: i64, element_span: Span, element_output: Option<Value>, expr: Option<String>) -> Self {
    //     ProfilerInfo2 {
    //         duration_sec,
    //         depth,
    //         element_span,
    //         element_output,
    //         expr,
    //         children: vec![]
    //     }
    // }

    // pub fn with_duration(&mut self) {
    //     self.duration_sec = self.start.elapsed().as_secs_f64();
    // }
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
    collect_exprs: bool,
    elements: Vec<ProfilerInfo2>,
    parents: Vec<ElementId>,
}

impl Profiler {
    pub fn new(
        max_depth: i64,
        collect_spans: bool,
        collect_source: bool,
        collect_expanded_source: bool,
        collect_values: bool,
        collect_exprs: bool,
        span: Span,
    ) -> Self {
        let first = ProfilerInfo2 {
            start: Instant::now(),
            duration_sec: 0.0,
            depth: 0,
            element_span: span,
            element_output: if collect_values {
                Some(Value::nothing(span))
            } else {
                None
            },
            expr: if collect_exprs {
                Some("call".to_string())
            } else {
                None
            },
            children: vec![],
        };

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
            collect_exprs,
            elements: vec![first],
            parents: vec![ElementId(0)],
        }
    }
}

impl Debugger for Profiler {
    fn enter_block(&mut self, engine_state: &EngineState, block: &Block) {
        // println!("- enter block {:?}", block.span);
        self.depth += 1;

        // last element becomes a parent node
        self.parents.push(ElementId(self.elements.len() - 1));
        println!(
            "- enter block {:?}, depth {}, new parent: {}",
            block
                .span
                .map(|span| String::from_utf8_lossy(engine_state.get_span_contents(span))),
            self.depth,
            self.elements.len() - 1
        );
    }

    fn leave_block(&mut self, engine_state: &EngineState, block: &Block) {
        self.depth -= 1;
        // println!("- leave block {:?}", block.span);

        self.parents.pop();
        println!(
            "- leave block {:?}, depth: {}, new parent: {:?}",
            block
                .span
                .map(|span| String::from_utf8_lossy(engine_state.get_span_contents(span))),
            self.depth,
            self.parents.last()
        );
    }

    fn enter_element(&mut self, engine_state: &EngineState, element: &PipelineElement) {
        // println!("- enter element {:?}", element.span());
        let source_fragment =
            String::from_utf8_lossy(engine_state.get_span_contents(element.span())).to_string();
        // println!("=== {source_fragment}; {:?}", element.expression().expr);
        if self.depth > self.max_depth {
            return;
        }

        self.element_start_times.push(Instant::now());

        let Some(parent_id) = self.parents.last() else {
            eprintln!("Internal Profiler Error: Missing parent element ID.");
            return;
        };

        let new_id = ElementId(self.elements.len());
        println!("- enter element {:?} id {}",
                 String::from_utf8_lossy(engine_state.get_span_contents(element.span())),
                 new_id.0);
        {
            self.elements
                .push(ProfilerInfo2::new(self.depth, element.span()));
        }

        let Some(mut parent) = self.elements.get_mut(parent_id.0) else {
            eprintln!("Internal Profiler Error: Missing parent element.");
            return;
        };

        parent.children.push(new_id);
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

        let duration = start.elapsed().as_secs_f64();

        let element_span = element.span();

        if self.collect_source {
            let source_fragment =
                String::from_utf8_lossy(engine_state.get_span_contents(element_span)).to_string();
            // println!("=== {source_fragment}; {:?}", element.expression().expr);
            self.source_fragments
                .insert((element_span.start, element_span.end), source_fragment);
        }

        let expr_opt = if self.collect_exprs {
            Some(match element {
                PipelineElement::Expression(_, expression) => {
                    expr_to_string(engine_state, &expression.expr)
                }
                _ => "other".to_string(),
            })
        } else {
            None
        };

        let out_opt = if self.collect_values {
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
            element_input: out_opt.clone(),
            expr: expr_opt.clone(),
        };

        self.element_durations_sec
            .push((info, start.elapsed().as_secs_f64()));
        // println!("- leave element {:?}", element.span());
        println!(
            "- leave element {:?} id {}",
            String::from_utf8_lossy(engine_state.get_span_contents(element.span())),
            self.elements.len() - 1
        );

        let Some(parent_id) = self.parents.last() else {
            eprintln!("Internal Profiler Error: Missing parent element ID.");
            return;
        };

        let Some(parent) = self.elements.get(parent_id.0) else {
            eprintln!("Internal Profiler Error: Missing parent element.");
            return;
        };

        let Some(last_element_id) = parent.children.last() else {
            eprintln!("Internal Profiler Error: Missing last element ID.");
            return;
        };

        let id = last_element_id.0;

        let Some(mut last_element) = self.elements.get_mut(id) else {
            eprintln!("Internal Profiler Error: Missing last element.");
            return;
        };

        last_element.duration_sec = last_element.start.elapsed().as_secs_f64();
        last_element.element_output = out_opt;
        last_element.expr = expr_opt;

        // self.elements.push(ProfilerInfo2::new(duration, self.depth, element_span, inp_opt, expr_opt));
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

            if let Some(expr_string) = &info.expr {
                row.push("expr", Value::string(expr_string.clone(), profiler_span));
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

        print_elements(self, *self.parents.first().unwrap());

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

fn expr_to_string(engine_state: &EngineState, expr: &Expr) -> String {
    match expr {
        Expr::Binary(_) => "binary".to_string(),
        Expr::BinaryOp(_, _, _) => "binary operation".to_string(),
        Expr::Block(_) => "block".to_string(),
        Expr::Bool(_) => "bool".to_string(),
        Expr::Call(call) => {
            let decl = engine_state.get_decl(call.decl_id);
            if decl.name() == "collect" && call.head == Span::new(0, 0) {
                "call (implicit collect)"
            } else {
                "call"
            }
            .to_string()
        }
        Expr::CellPath(_) => "cell path".to_string(),
        Expr::Closure(_) => "closure".to_string(),
        Expr::DateTime(_) => "datetime".to_string(),
        Expr::Directory(_, _) => "directory".to_string(),
        Expr::ExternalCall(_, _, _) => "external call".to_string(),
        Expr::Filepath(_, _) => "filepath".to_string(),
        Expr::Float(_) => "float".to_string(),
        Expr::FullCellPath(full_cell_path) => {
            let head = expr_to_string(engine_state, &full_cell_path.head.expr);
            format!("full cell path ({head})")
        }
        Expr::Garbage => "garbage".to_string(),
        Expr::GlobPattern(_, _) => "glob pattern".to_string(),
        Expr::ImportPattern(_) => "import pattern".to_string(),
        Expr::Int(_) => "int".to_string(),
        Expr::Keyword(_, _, _) => "keyword".to_string(),
        Expr::List(_) => "list".to_string(),
        Expr::MatchBlock(_) => "match block".to_string(),
        Expr::Nothing => "nothing".to_string(),
        Expr::Operator(_) => "operator".to_string(),
        Expr::Overlay(_) => "overlay".to_string(),
        Expr::Range(_, _, _, _) => "range".to_string(),
        Expr::Record(_) => "record".to_string(),
        Expr::RowCondition(_) => "row condition".to_string(),
        Expr::Signature(_) => "signature".to_string(),
        Expr::Spread(_) => "spread".to_string(),
        Expr::String(_) => "string".to_string(),
        Expr::StringInterpolation(_) => "string interpolation".to_string(),
        Expr::Subexpression(_) => "subexpression".to_string(),
        Expr::Table(_, _) => "table".to_string(),
        Expr::UnaryNot(_) => "unary not".to_string(),
        Expr::ValueWithUnit(_, _) => "value with unit".to_string(),
        Expr::Var(_) => "var".to_string(),
        Expr::VarDecl(_) => "var decl".to_string(),
    }
}

fn print_elements(profiler: &Profiler, element_id: ElementId) {
    let element = &profiler.elements[element_id.0];
    let indent = "  ".repeat(element.depth as usize);
    println!(
        "{} {} id {} {:?} {:?} {}",
        indent,
        element.depth,
        element_id.0,
        element.element_span,
        profiler
            .source_fragments
            .get(&(element.element_span.start, element.element_span.end)),
        element.duration_sec
    );

    for child in &element.children {
        print_elements(profiler, *child);
    }
}
