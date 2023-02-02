use std::time::Instant;

use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Argument, Block, Call, Expr};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct Inspect;

impl Command for Inspect {
    fn name(&self) -> &str {
        "inspect"
    }

    fn usage(&self) -> &str {
        "Inspect the running closure"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("inspect")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
            )
            .input_output_types(vec![
                (Type::Any, Type::Duration),
                (Type::Nothing, Type::Duration),
            ])
            .allow_variants_without_examples(true)
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let mut stack = stack.captures_to_stack(&capture_block.captures);

        // In order to provide the pipeline as a positional, it must be converted into a value.
        // But because pipelines do not have Clone, this one has to be cloned as a value
        // and then converted back into a pipeline for eval_block().
        // So, the metadata must be saved here and restored at that point.
        let input_metadata = input.metadata();
        let input_val = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input_val.clone());
            }
        }

        let elements = get_pipeline_elements(engine_state, &block)?;
        // eprintln!("Pipeline Elements: {:?}", elements);
        for el in elements {
            eprintln!("{{ {el} }}");
        }

        // Get the start time after all other computation has been done.
        let start_time = Instant::now();
        eval_block(
            engine_state,
            &mut stack,
            block,
            input_val.into_pipeline_data_with_metadata(input_metadata),
            redirect_stdout,
            redirect_stderr,
        )?
        .into_value(call.head);

        let end_time = Instant::now();

        let output = Value::Duration {
            val: (end_time - start_time).as_nanos() as i64,
            span: call.head,
        };

        Ok(output.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Inspect a command within a closure",
                example: "inspect { sleep 500ms }",
                result: None,
            },
            Example {
                description: "Inspect a command using an existing input",
                example: "http get https://www.nushell.sh/book/ | inspect { split chars }",
                result: None,
            },
        ]
    }
}

pub fn get_pipeline_elements(
    engine_state: &EngineState,
    // stack: &mut Stack,
    block: &Block,
    // mut input: PipelineData,
    // redirect_stdout: bool,
    // redirect_stderr: bool,
) -> Result<Vec<String>, ShellError> {
    let mut elements = vec![];
    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        let mut i = 0;
        while i < pipeline.elements.len() {
            let pipeline_element = &pipeline.elements[i];
            let pipeline_expression = pipeline_element.expression().clone();
            let pipeline_span = &pipeline_element.span();
            let element_str =
                String::from_utf8_lossy(engine_state.get_span_contents(pipeline_span));
            let value = Value::string(element_str.to_string(), *pipeline_span);
            let expr = pipeline_expression.expr.clone();
            let (command_name, command_args) = if let Expr::Call(c) = expr {
                let command = engine_state.get_decl(c.decl_id);
                (command.name().to_string(), get_arguments(c))
            } else {
                ("no-op".to_string(), "no-args".to_string())
            };
            let index = format!("{pipeline_idx}_{i}");
            let value_type = value.get_type();
            let value_span_start = value.span()?.start;
            let value_span_end = value.span()?.end;
            let command_name = command_name;
            let element = format!("\"index\": \"{index}\", \"value_type\": \"{value_type}\", \"value_span_start\": {value_span_start}, \"value_span_end\": {value_span_end}, \"command_name\": \"{command_name}\", \"arguments\": {{ {command_args} }}");
            elements.push(element);
            i += 1;
        }
    }
    Ok(elements)
}

fn get_arguments(call: Box<Call>) -> String {
    let mut arguments: Vec<String> = Vec::new();
    for arg in &call.arguments {
        match arg {
            Argument::Named((name, something, opt_expr)) => {
                let arg_type = "named";
                let arg_name = name.item.clone();
                let arg_name_span_start = name.span.start;
                let arg_name_span_end = name.span.end;
                arguments.push(format!(
                    "\"arg_type\": \"{arg_type}\", \"arg_name\": \"{arg_name}\", \"start\": {arg_name_span_start}, \"end\": {arg_name_span_end}"
                ));

                let some_thing = if let Some(thing) = something {
                    let thing_type = "thing";
                    let thing_name = thing.item.clone();
                    let thing_span_start = thing.span.start;
                    let thing_span_end = thing.span.end;
                    format!(
                        "\"thing_type\": \"{thing_type}\", \"thing_name\": \"{thing_name}\", \"start\": {thing_span_start}, \"end\": {thing_span_end}"
                    )
                } else {
                    format!("\"thing_type\": \"thing\", \"thing_value\": \"None\"")
                };
                arguments.push(some_thing);

                let some_expr = if let Some(expression) = opt_expr {
                    let expr_type = "expr";
                    let expr_name = expression.expr.clone();
                    let expr_span_start = expression.span.start;
                    let expr_span_end = expression.span.end;
                    format!(
                        "\"expr_type\": \"{expr_type}\", \"expr_name\": \"{expr_name:?}\", \"start\": {expr_span_start}, \"end\": {expr_span_end}"
                    )
                } else {
                    format!("\"expr_type\": \"expr\", \"expr_value\": \"None\"")
                };
                arguments.push(some_expr);
            }
            Argument::Positional(inner_expr) => {
                let arg_type = "positional";
                let (arg_value_type, arg_value, span_start, span_end) = match inner_expr
                    .expr
                    .clone()
                {
                    Expr::String(s) => ("string", s, inner_expr.span.start, inner_expr.span.end),
                    Expr::Bool(b) => (
                        "bool",
                        b.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Int(i) => (
                        "int",
                        i.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Float(f) => (
                        "float",
                        f.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Binary(b) => (
                        "binary",
                        String::from_utf8_lossy(&b).to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Range(r, s, e, t) => (
                        "range",
                        "range".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Var(v) => (
                        "var",
                        v.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::VarDecl(v) => (
                        "var_decl",
                        v.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Call(c) => (
                        "call",
                        "call".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::ExternalCall(e, s, t) => (
                        "external_call",
                        "external_call".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Operator(o) => (
                        "operator",
                        o.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::RowCondition(r) => (
                        "row_condition",
                        r.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::UnaryNot(u) => (
                        "unary_not",
                        "unary_not".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::BinaryOp(b, o, e) => (
                        "binary_op",
                        "binary_op".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Subexpression(s) => (
                        "subexpression",
                        s.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Block(b) => (
                        "block",
                        b.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Closure(c) => (
                        "closure",
                        c.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::List(l) => (
                        "list",
                        "list".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Table(t, s) => (
                        "table",
                        "table".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Record(r) => (
                        "record",
                        "record".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Keyword(k, s, e) => (
                        "keyword",
                        String::from_utf8_lossy(&k).to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::ValueWithUnit(v, u) => (
                        "value_with_unit",
                        "value_with_unit".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::DateTime(d) => (
                        "datetime",
                        d.to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Filepath(f) => {
                        ("filepath", f, inner_expr.span.start, inner_expr.span.end)
                    }
                    Expr::Directory(d) => {
                        ("directory", d, inner_expr.span.start, inner_expr.span.end)
                    }
                    Expr::GlobPattern(g) => (
                        "glob_pattern",
                        g,
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::CellPath(c) => (
                        "cell_path",
                        "cell_path".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::FullCellPath(f) => (
                        "full_cell_path",
                        "full_cell_path".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::ImportPattern(i) => (
                        "import_pattern",
                        "import_pattern".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Overlay(o) => (
                        "overlay",
                        "overlay".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Signature(s) => (
                        "signature",
                        "signature".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::StringInterpolation(s) => (
                        "string_interpolation",
                        "string_interpolation".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Nothing => (
                        "nothing",
                        "nothing".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                    Expr::Garbage => (
                        "garbage",
                        "garbage".to_string(),
                        inner_expr.span.start,
                        inner_expr.span.end,
                    ),
                };
                arguments.push(format!("\"arg_type\": \"{arg_type}\", \"arg_name\": \"{arg_value}\", \"arg_type\": \"{arg_value_type}\", \"start\": {span_start}, \"end\": {span_end}"));
            }
            Argument::Unknown(inner_expr) => {
                arguments.push(format!("\"e_unknown\": {inner_expr:#?}"))
            }
        };
    }

    arguments.join(", ")
}
