#![allow(unused_imports)]
use nu_protocol::{
    ast::{
        Argument, Assignment, Bits, Block, Boolean, Call, Comparison, Expr, Expression, Math,
        Operator, PathMember, PipelineElement, Redirection,
    },
    engine::{EngineState, ProfilingConfig, Stack},
    record, Config, DataSource, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineMetadata, Range, Record, ShellError, Span, Spanned, Unit, Value, VarId,
    ENV_VARIABLE_ID,
};

use crate::CallExt;

// render a call into a string representation for debugging purposes
//
// Ugly -- requires redundant evaluation of the arguments.
pub fn dbg_trace_call(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<String, ShellError> {
    let mut argv: Vec<String> = Vec::new();
    argv.push(
        span_string(engine_state, call.span()), //String::from_utf8_lossy(engine_state.get_span_contents(call.span())).to_string()
    );

    for p in call.named_iter() {
        argv.push(match p {
            (named, _, Some(_val)) => {
                let name = &named.item;
                format!(
                    "--{} {}",
                    name,
                    dbg_value(call.get_flag(engine_state, stack, name)?)
                )
            }
            (_named, _, None) => "".to_string(),
        });
    }

    Ok(argv.join(" "))
}

pub fn dbg_trace_pipeline_element(
    engine_state: &EngineState,
    _stack: &Stack,
    element: &PipelineElement,
) -> Result<String, ShellError> {
    Ok(span_string(engine_state, element.span()))
}

// stringification of Value, abbreviated to suit
fn dbg_value(value: Option<Value>) -> String {
    if let Some(v) = value {
        v.debug_string("", &Config::default())
            .escape_default()
            .to_string()
    } else {
        "".to_string()
    }
}

// format string with source file: line_number content_of_span
fn span_string(engine_state: &EngineState, span: Span) -> String {
    let filename = if let Some(f) = engine_state.get_file_for_span(span) {
        f
    } else {
        "unknown"
    };

    format!(
        "{}:{:<5} {}",
        filename,
        engine_state.get_line_number(span),
        &String::from_utf8_lossy(engine_state.get_span_contents(span)).to_string()
    )
}
