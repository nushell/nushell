//! Nushell Profiler
//!
//! Profiler implements the Debugger trait and can be used via the `debug profile` command for
//! profiling Nushell code.

use crate::{
    PipelineData, PipelineExecutionData, ShellError, Span, Value,
    ast::{Block, Expr, PipelineElement},
    debugger::Debugger,
    engine::EngineState,
    ir::IrBlock,
    record,
};
use std::{borrow::Borrow, io::BufRead};
use web_time::Instant;

#[derive(Debug, Clone, Copy)]
struct ElementId(usize);

/// Stores profiling information about one pipeline element
#[derive(Debug, Clone)]
struct ElementInfo {
    start: Instant,
    duration_ns: i64,
    depth: i64,
    element_span: Span,
    element_output: Option<Value>,
    expr: Option<String>,
    instruction: Option<(usize, String)>,
    children: Vec<ElementId>,
}

impl ElementInfo {
    pub fn new(depth: i64, element_span: Span) -> Self {
        ElementInfo {
            start: Instant::now(),
            duration_ns: 0,
            depth,
            element_span,
            element_output: None,
            expr: None,
            instruction: None,
            children: vec![],
        }
    }
}

/// Whether [`Profiler`] should report duration as [`Value::Duration`]
#[derive(Debug, Clone, Copy)]
pub enum DurationMode {
    Milliseconds,
    Value,
}

/// Options for [`Profiler`]
#[derive(Debug, Clone)]
pub struct ProfilerOptions {
    pub max_depth: i64,
    pub collect_spans: bool,
    pub collect_source: bool,
    pub collect_expanded_source: bool,
    pub collect_values: bool,
    pub collect_exprs: bool,
    pub collect_instructions: bool,
    pub collect_lines: bool,
    pub duration_mode: DurationMode,
}

/// Basic profiler, used in `debug profile`
#[derive(Debug, Clone)]
pub struct Profiler {
    depth: i64,
    opts: ProfilerOptions,
    elements: Vec<ElementInfo>,
    element_stack: Vec<ElementId>,
}

impl Profiler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(opts: ProfilerOptions, span: Span) -> Self {
        let first = ElementInfo {
            start: Instant::now(),
            duration_ns: 0,
            depth: 0,
            element_span: span,
            element_output: opts.collect_values.then(|| Value::nothing(span)),
            expr: opts.collect_exprs.then(|| "call".to_string()),
            instruction: opts
                .collect_instructions
                .then(|| (0, "<start>".to_string())),
            children: vec![],
        };

        Profiler {
            depth: 0,
            opts,
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

        root_element.duration_ns = root_element.start.elapsed().as_nanos() as i64;
    }

    fn enter_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.depth += 1;
    }

    fn leave_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.depth -= 1;
    }

    fn enter_element(&mut self, engine_state: &EngineState, element: &PipelineElement) {
        if self.depth > self.opts.max_depth {
            return;
        }

        let Some(parent_id) = self.last_element_id() else {
            eprintln!("Profiler Error: Missing parent element ID.");
            return;
        };

        let expr_opt = self
            .opts
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
        result: &Result<PipelineData, ShellError>,
    ) {
        if self.depth > self.opts.max_depth {
            return;
        }

        let element_span = element.expr.span;

        let out_opt = self.opts.collect_values.then(|| match result {
            Ok(pipeline_data) => match pipeline_data {
                PipelineData::Value(val, ..) => val.clone(),
                PipelineData::ListStream(..) => Value::string("list stream", element_span),
                PipelineData::ByteStream(..) => Value::string("byte stream", element_span),
                _ => Value::nothing(element_span),
            },
            Err(e) => Value::error(e.clone(), element_span),
        });

        let Some(last_element) = self.last_element_mut() else {
            eprintln!("Profiler Error: Missing last element.");
            return;
        };

        last_element.duration_ns = last_element.start.elapsed().as_nanos() as i64;
        last_element.element_output = out_opt;

        self.element_stack.pop();
    }

    fn enter_instruction(
        &mut self,
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        _registers: &[PipelineExecutionData],
    ) {
        if self.depth > self.opts.max_depth {
            return;
        }

        let Some(parent_id) = self.last_element_id() else {
            eprintln!("Profiler Error: Missing parent element ID.");
            return;
        };

        let instruction = &ir_block.instructions[instruction_index];
        let span = ir_block.spans[instruction_index];

        let instruction_opt = self.opts.collect_instructions.then(|| {
            (
                instruction_index,
                instruction
                    .display(engine_state, &ir_block.data)
                    .to_string(),
            )
        });

        let new_id = ElementId(self.elements.len());

        let mut new_element = ElementInfo::new(self.depth, span);
        new_element.instruction = instruction_opt;

        self.elements.push(new_element);

        let Some(parent) = self.elements.get_mut(parent_id.0) else {
            eprintln!("Profiler Error: Missing parent element.");
            return;
        };

        parent.children.push(new_id);
        self.element_stack.push(new_id);
    }

    fn leave_instruction(
        &mut self,
        _engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        error: Option<&ShellError>,
    ) {
        if self.depth > self.opts.max_depth {
            return;
        }

        let instruction = &ir_block.instructions[instruction_index];
        let span = ir_block.spans[instruction_index];

        let out_opt = self
            .opts
            .collect_values
            .then(|| {
                error
                    .map(Err)
                    .or_else(|| {
                        instruction
                            .output_register()
                            .map(|register| Ok(&registers[register.get() as usize]))
                    })
                    .map(|result| format_result(result.map(|r| &r.body), span))
            })
            .flatten();

        let Some(last_element) = self.last_element_mut() else {
            eprintln!("Profiler Error: Missing last element.");
            return;
        };

        last_element.duration_ns = last_element.start.elapsed().as_nanos() as i64;
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
        Expr::AttributeBlock(ab) => expr_to_string(engine_state, &ab.item.expr),
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
        Expr::String(_) | Expr::RawString(_) => "string".to_string(),
        Expr::StringInterpolation(_) => "string interpolation".to_string(),
        Expr::GlobInterpolation(_, _) => "glob interpolation".to_string(),
        Expr::Collect(_, _) => "collect".to_string(),
        Expr::Subexpression(_) => "subexpression".to_string(),
        Expr::Table(_) => "table".to_string(),
        Expr::UnaryNot(_) => "unary not".to_string(),
        Expr::ValueWithUnit(_) => "value with unit".to_string(),
        Expr::Var(_) => "var".to_string(),
        Expr::VarDecl(_) => "var decl".to_string(),
    }
}

fn format_result(
    result: Result<&PipelineData, impl Borrow<ShellError>>,
    element_span: Span,
) -> Value {
    match result {
        Ok(pipeline_data) => match pipeline_data {
            PipelineData::Value(val, ..) => val.clone(),
            PipelineData::ListStream(..) => Value::string("list stream", element_span),
            PipelineData::ByteStream(..) => Value::string("byte stream", element_span),
            _ => Value::nothing(element_span),
        },
        Err(e) => Value::error(e.borrow().clone(), element_span),
    }
}

// Find a file name and a line number (indexed from 1) of a span
fn find_file_of_span(engine_state: &EngineState, span: Span) -> Option<(&str, usize)> {
    for file in engine_state.files() {
        if file.covered_span.contains_span(span) {
            // count the number of lines between file start and the searched span start
            let chunk =
                engine_state.get_span_contents(Span::new(file.covered_span.start, span.start));
            let nlines = chunk.lines().count();
            // account for leading part of current line being counted as a separate line
            let line_num = if chunk.last() == Some(&b'\n') {
                nlines + 1
            } else {
                nlines
            };

            // first line has no previous line, clamp up to `1`
            let line_num = usize::max(line_num, 1);

            return Some((&file.name, line_num));
        }
    }

    None
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

    if profiler.opts.collect_lines {
        if let Some((fname, line_num)) = find_file_of_span(engine_state, element.element_span) {
            row.push("file", Value::string(fname, profiler_span));
            row.push("line", Value::int(line_num as i64, profiler_span));
        } else {
            row.push("file", Value::nothing(profiler_span));
            row.push("line", Value::nothing(profiler_span));
        }
    }

    if profiler.opts.collect_spans {
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

    if profiler.opts.collect_source {
        let val = String::from_utf8_lossy(engine_state.get_span_contents(element.element_span));
        let val = val.trim();
        let nlines = val.lines().count();

        let fragment = if profiler.opts.collect_expanded_source {
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

    if let Some((instruction_index, instruction)) = &element.instruction {
        row.push(
            "pc",
            (*instruction_index)
                .try_into()
                .map(|index| Value::int(index, profiler_span))
                .unwrap_or(Value::nothing(profiler_span)),
        );
        row.push("instruction", Value::string(instruction, profiler_span));
    }

    if let Some(val) = &element.element_output {
        row.push("output", val.clone());
    }

    match profiler.opts.duration_mode {
        DurationMode::Milliseconds => {
            let val = Value::float(element.duration_ns as f64 / 1000.0 / 1000.0, profiler_span);
            row.push("duration_ms", val);
        }
        DurationMode::Value => {
            let val = Value::duration(element.duration_ns, profiler_span);
            row.push("duration", val);
        }
    };

    let mut rows = vec![Value::record(row, profiler_span)];

    for child in &element.children {
        let child_rows = collect_data(engine_state, profiler, *child, element_id, profiler_span)?;
        rows.extend(child_rows);
    }

    Ok(rows)
}
