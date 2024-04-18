//! Nushell Profiler
//!
//! Profiler implements the Debugger trait and can be used via the `debug profile` command for
//! profiling Nushell code.

use crate::{
    ast::{Block, Expr, PipelineElement},
    debugger::Debugger,
    engine::EngineState,
    record, PipelineData, ShellError, Span, Value,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct ElementId(usize);

/// Stores profiling information about one pipeline element
#[derive(Debug, Clone)]
struct ElementInfo {
    start: Instant,
    duration_sec: f64,
    depth: i64,
    element_span: Span,
    element_output: Option<Value>,
    expr: Option<String>,
    children: Vec<ElementId>,
}

impl ElementInfo {
    pub fn new(depth: i64, element_span: Span) -> Self {
        ElementInfo {
            start: Instant::now(),
            duration_sec: 0.0,
            depth,
            element_span,
            element_output: None,
            expr: None,
            children: vec![],
        }
    }
}

/// Basic profiler, used in `debug profile`
#[derive(Debug, Clone)]
pub struct Profiler {
    depth: i64,
    max_depth: i64,
    collect_spans: bool,
    collect_source: bool,
    collect_expanded_source: bool,
    collect_values: bool,
    collect_exprs: bool,
    elements: Vec<ElementInfo>,
    element_stack: Vec<ElementId>,
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
        let first = ElementInfo {
            start: Instant::now(),
            duration_sec: 0.0,
            depth: 0,
            element_span: span,
            element_output: collect_values.then(|| Value::nothing(span)),
            expr: collect_exprs.then(|| "call".to_string()),
            children: vec![],
        };

        Profiler {
            depth: 0,
            max_depth,
            collect_spans,
            collect_source,
            collect_expanded_source,
            collect_values,
            collect_exprs,
            elements: vec![first],
            element_stack: vec![ElementId(0)],
        }
    }

    fn last_element_id(&self) -> Option<ElementId> {
        self.element_stack.last().copied()
    }

    fn last_element_mut(&mut self) -> Option<&mut ElementInfo> {
        self.last_element_id()
            .and_then(|id| self.elements.get_mut(id.0))
    }
}

impl Debugger for Profiler {
    fn activate(&mut self) {
        let Some(root_element) = self.last_element_mut() else {
            eprintln!("Profiler Error: Missing root element.");
            return;
        };

        root_element.start = Instant::now();
    }

    fn deactivate(&mut self) {
        let Some(root_element) = self.last_element_mut() else {
            eprintln!("Profiler Error: Missing root element.");
            return;
        };

        root_element.duration_sec = root_element.start.elapsed().as_secs_f64();
    }

    fn enter_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.depth += 1;
    }

    fn leave_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.depth -= 1;
    }

    fn enter_element(&mut self, engine_state: &EngineState, element: &PipelineElement) {
        if self.depth > self.max_depth {
            return;
        }

        let Some(parent_id) = self.last_element_id() else {
            eprintln!("Profiler Error: Missing parent element ID.");
            return;
        };

        let expr_opt = self
            .collect_exprs
            .then(|| expr_to_string(engine_state, &element.expr.expr));

        let new_id = ElementId(self.elements.len());

        let mut new_element = ElementInfo::new(self.depth, element.expr.span);
        new_element.expr = expr_opt;

        self.elements.push(new_element);

        let Some(parent) = self.elements.get_mut(parent_id.0) else {
            eprintln!("Profiler Error: Missing parent element.");
            return;
        };

        parent.children.push(new_id);
        self.element_stack.push(new_id);
    }

    fn leave_element(
        &mut self,
        _engine_state: &EngineState,
        element: &PipelineElement,
        result: &Result<(PipelineData, bool), ShellError>,
    ) {
        if self.depth > self.max_depth {
            return;
        }

        let element_span = element.expr.span;

        let out_opt = self.collect_values.then(|| match result {
            Ok((pipeline_data, _not_sure_what_this_is)) => match pipeline_data {
                PipelineData::Value(val, ..) => val.clone(),
                PipelineData::ListStream(..) => Value::string("list stream", element_span),
                PipelineData::ExternalStream { .. } => {
                    Value::string("external stream", element_span)
                }
                _ => Value::nothing(element_span),
            },
            Err(e) => Value::error(e.clone(), element_span),
        });

        let Some(last_element) = self.last_element_mut() else {
            eprintln!("Profiler Error: Missing last element.");
            return;
        };

        last_element.duration_sec = last_element.start.elapsed().as_secs_f64();
        last_element.element_output = out_opt;

        self.element_stack.pop();
    }

    fn report(&self, engine_state: &EngineState, profiler_span: Span) -> Result<Value, ShellError> {
        Ok(Value::list(
            collect_data(
                engine_state,
                self,
                ElementId(0),
                ElementId(0),
                profiler_span,
            )?,
            profiler_span,
        ))
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
        Expr::ExternalCall(_, _) => "external call".to_string(),
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
        Expr::Keyword(_) => "keyword".to_string(),
        Expr::List(_) => "list".to_string(),
        Expr::MatchBlock(_) => "match block".to_string(),
        Expr::Nothing => "nothing".to_string(),
        Expr::Operator(_) => "operator".to_string(),
        Expr::Overlay(_) => "overlay".to_string(),
        Expr::Range(_) => "range".to_string(),
        Expr::Record(_) => "record".to_string(),
        Expr::RowCondition(_) => "row condition".to_string(),
        Expr::Signature(_) => "signature".to_string(),
        Expr::String(_) => "string".to_string(),
        Expr::StringInterpolation(_) => "string interpolation".to_string(),
        Expr::Subexpression(_) => "subexpression".to_string(),
        Expr::Table(_) => "table".to_string(),
        Expr::UnaryNot(_) => "unary not".to_string(),
        Expr::ValueWithUnit(_) => "value with unit".to_string(),
        Expr::Var(_) => "var".to_string(),
        Expr::VarDecl(_) => "var decl".to_string(),
    }
}

fn collect_data(
    engine_state: &EngineState,
    profiler: &Profiler,
    element_id: ElementId,
    parent_id: ElementId,
    profiler_span: Span,
) -> Result<Vec<Value>, ShellError> {
    let element = &profiler.elements[element_id.0];

    let mut row = record! {
        "depth" => Value::int(element.depth, profiler_span),
        "id" => Value::int(element_id.0 as i64, profiler_span),
        "parent_id" => Value::int(parent_id.0 as i64, profiler_span),
    };

    if profiler.collect_spans {
        let span_start = i64::try_from(element.element_span.start)
            .map_err(|_| profiler_error("error converting span start to i64", profiler_span))?;
        let span_end = i64::try_from(element.element_span.end)
            .map_err(|_| profiler_error("error converting span end to i64", profiler_span))?;

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

    if profiler.collect_source {
        let val = String::from_utf8_lossy(engine_state.get_span_contents(element.element_span));
        let val = val.trim();
        let nlines = val.lines().count();

        let fragment = if profiler.collect_expanded_source {
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

    if let Some(expr_string) = &element.expr {
        row.push("expr", Value::string(expr_string.clone(), profiler_span));
    }

    if let Some(val) = &element.element_output {
        row.push("output", val.clone());
    }

    row.push(
        "duration_ms",
        Value::float(element.duration_sec * 1e3, profiler_span),
    );

    let mut rows = vec![Value::record(row, profiler_span)];

    for child in &element.children {
        let child_rows = collect_data(engine_state, profiler, *child, element_id, profiler_span)?;
        rows.extend(child_rows);
    }

    Ok(rows)
}
