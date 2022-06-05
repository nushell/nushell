use crate::{current_dir_str, get_full_help};
use nu_path::expand_path_with;
use nu_protocol::{
    ast::{Block, Call, Expr, Expression, Operator},
    engine::{EngineState, Stack, Visibility},
    IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Range, ShellError, Span,
    Spanned, SyntaxShape, Unit, Value, VarId, ENV_VARIABLE_ID,
};
use nu_utils::stdout_write_all_and_flush;
use std::cmp::Ordering;
use std::collections::HashMap;
use sysinfo::SystemExt;

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(operator.clone()),
        Expression { span, expr, .. } => {
            Err(ShellError::UnknownOperator(format!("{:?}", expr), *span))
        }
    }
}

pub fn eval_call(
    engine_state: &EngineState,
    caller_stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    if let Some(ctrlc) = &engine_state.ctrlc {
        if ctrlc.load(core::sync::atomic::Ordering::SeqCst) {
            return Ok(Value::Nothing.into_pipeline_data());
        }
    }
    let decl = engine_state.get_decl(call.decl_id);

    if !decl.is_known_external() && call.named_iter().any(|(flag, _, _)| flag.item == "help") {
        let mut signature = decl.signature();
        signature.usage = decl.usage().to_string();
        signature.extra_usage = decl.extra_usage().to_string();

        let full_help = get_full_help(
            &signature,
            &decl.examples(),
            engine_state,
            caller_stack,
            call.head,
        );
        Ok(Value::String(full_help).into_pipeline_data())
    } else if let Some(block_id) = decl.get_block_id() {
        let block = engine_state.get_block(block_id);

        let mut callee_stack = caller_stack.gather_captures(&block.captures, call.head);

        for (param_idx, param) in decl
            .signature()
            .required_positional
            .iter()
            .chain(decl.signature().optional_positional.iter())
            .enumerate()
        {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            if let Some(arg) = call.positional_nth(param_idx) {
                let result = eval_expression(engine_state, caller_stack, arg)?;
                callee_stack.add_var(var_id, result);
            } else if let Some(arg) = &param.default_value {
                let result = eval_expression(engine_state, caller_stack, arg)?;
                callee_stack.add_var(var_id, result);
            } else {
                callee_stack.add_var(var_id, Value::Nothing);
            }
        }

        if let Some(rest_positional) = decl.signature().rest_positional {
            let mut rest_items = vec![];

            for arg in call.positional_iter().skip(
                decl.signature().required_positional.len()
                    + decl.signature().optional_positional.len(),
            ) {
                let result = eval_expression(engine_state, caller_stack, arg)?;
                rest_items.push(result);
            }

            let span = call.head;

            callee_stack.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lacks var_id"),
                Value::List(rest_items),
            )
        }

        for named in decl.signature().named {
            if let Some(var_id) = named.var_id {
                let mut found = false;
                for call_named in call.named_iter() {
                    if call_named.0.item == named.long {
                        if let Some(arg) = &call_named.2 {
                            let result = eval_expression(engine_state, caller_stack, arg)?;

                            callee_stack.add_var(var_id, result);
                        } else if let Some(arg) = &named.default_value {
                            let result = eval_expression(engine_state, caller_stack, arg)?;

                            callee_stack.add_var(var_id, result);
                        } else {
                            callee_stack.add_var(var_id, Value::Bool(true))
                        }
                        found = true;
                    }
                }

                if !found {
                    if named.arg.is_none() {
                        callee_stack.add_var(var_id, Value::Bool(false))
                    } else if let Some(arg) = &named.default_value {
                        let result = eval_expression(engine_state, caller_stack, arg)?;

                        callee_stack.add_var(var_id, result);
                    } else {
                        callee_stack.add_var(var_id, Value::Nothing)
                    }
                }
            }
        }

        let result = eval_block(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stderr,
        );

        if block.redirect_env {
            let caller_env_vars = caller_stack.get_env_var_names(engine_state);

            // remove env vars that are present in the caller but not in the callee
            // (the callee hid them)
            for var in caller_env_vars.iter() {
                if !callee_stack.has_env_var(engine_state, var) {
                    caller_stack.remove_env_var(engine_state, var);
                }
            }

            // add new env vars from callee to caller
            for (var, value) in callee_stack.get_stack_env_vars() {
                caller_stack.add_env_var(var, value);
            }
        }

        result
    } else {
        // We pass caller_stack here with the knowledge that internal commands
        // are going to be specifically looking for global state in the stack
        // rather than any local state.
        decl.run(engine_state, caller_stack, call, input)
    }
}

fn eval_external(
    engine_state: &EngineState,
    stack: &mut Stack,
    head: &Expression,
    args: &[Expression],
    input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    let decl_id = engine_state
        .find_decl("run-external".as_bytes(), &[])
        .ok_or(ShellError::ExternalNotSupported(head.span))?;

    let command = engine_state.get_decl(decl_id);

    let mut call = Call::new(head.span);

    call.add_positional(head.clone());

    for arg in args {
        call.add_positional(arg.clone())
    }

    if redirect_stdout {
        call.add_named((
            Spanned {
                item: "redirect-stdout".into(),
                span: head.span,
            },
            None,
            None,
        ))
    }

    if redirect_stderr {
        call.add_named((
            Spanned {
                item: "redirect-stderr".into(),
                span: head.span,
            },
            None,
            None,
        ))
    }

    command.run(engine_state, stack, &call, input)
}

pub fn eval_expression(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
) -> Result<Value, ShellError> {
    match &expr.expr {
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Int(i) => Ok(Value::Int(*i)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::Binary(b) => Ok(Value::Binary(b.clone())),
        Expr::ValueWithUnit(e, unit) => match eval_expression(engine_state, stack, e)? {
            Value::Int(val) => Ok(compute(val, unit.item, unit.span)),
            x => Err(ShellError::CantConvert(
                "unit value".into(),
                x.get_type().to_string(),
                e.span,
                None,
            )),
        },
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                eval_expression(engine_state, stack, f)?
            } else {
                Value::Nothing
            };

            let next = if let Some(s) = next {
                eval_expression(engine_state, stack, s)?
            } else {
                Value::Nothing
            };

            let to = if let Some(t) = to {
                eval_expression(engine_state, stack, t)?
            } else {
                Value::Nothing
            };

            Ok(Value::Range(Box::new(Range::new(
                expr.span, from, next, to, operator,
            )?)))
        }
        Expr::Var(var_id) => eval_variable(engine_state, stack, *var_id, expr.span),
        Expr::VarDecl(_) => Ok(Value::Nothing),
        Expr::CellPath(cell_path) => Ok(Value::CellPath(cell_path.clone())),
        Expr::FullCellPath(cell_path) => {
            let value = eval_expression(engine_state, stack, &cell_path.head)?;

            value.follow_cell_path(&cell_path.tail, false)
        }
        Expr::ImportPattern(_) => Ok(Value::Nothing),
        Expr::Call(call) => {
            // FIXME: protect this collect with ctrl-c
            Ok(eval_call(engine_state, stack, call, PipelineData::new())?.into_value(call.head))
        }
        Expr::ExternalCall(head, args) => {
            let span = head.span;
            // FIXME: protect this collect with ctrl-c
            Ok(eval_external(
                engine_state,
                stack,
                head,
                args,
                PipelineData::new(),
                false,
                false,
            )?
            .into_value(span))
        }
        Expr::DateTime(dt) => Ok(Value::Date(*dt)),
        Expr::Operator(_) => Ok(Value::Nothing),
        Expr::UnaryNot(expr) => {
            let lhs = eval_expression(engine_state, stack, expr)?;
            match lhs {
                Value::Bool(val) => Ok(Value::Bool(!val)),
                _ => Err(ShellError::TypeMismatch("bool".to_string(), expr.span)),
            }
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let op_span = op.span;
            let lhs = eval_expression(engine_state, stack, lhs)?;
            let op = eval_operator(op)?;

            match op {
                Operator::And => {
                    if !lhs.is_true() {
                        Ok(Value::Bool(false))
                    } else {
                        let rhs = eval_expression(engine_state, stack, rhs)?;
                        lhs.and(op_span, &rhs, expr.span)
                    }
                }
                Operator::Or => {
                    if lhs.is_true() {
                        Ok(Value::Bool(true))
                    } else {
                        let rhs = eval_expression(engine_state, stack, rhs)?;
                        lhs.or(op_span, &rhs, expr.span)
                    }
                }
                Operator::Plus => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.add(op_span, &rhs, expr.span)
                }
                Operator::Minus => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.sub(op_span, &rhs, expr.span)
                }
                Operator::Multiply => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.mul(op_span, &rhs, expr.span)
                }
                Operator::Divide => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.div(op_span, &rhs, expr.span)
                }
                Operator::LessThan => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.lt(op_span, &rhs, expr.span)
                }
                Operator::LessThanOrEqual => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.lte(op_span, &rhs, expr.span)
                }
                Operator::GreaterThan => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.gt(op_span, &rhs, expr.span)
                }
                Operator::GreaterThanOrEqual => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.gte(op_span, &rhs, expr.span)
                }
                Operator::Equal => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.eq(op_span, &rhs, expr.span)
                }
                Operator::NotEqual => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.ne(op_span, &rhs, expr.span)
                }
                Operator::In => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.r#in(op_span, &rhs, expr.span)
                }
                Operator::NotIn => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.not_in(op_span, &rhs, expr.span)
                }
                Operator::RegexMatch => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.regex_match(op_span, &rhs, false, expr.span)
                }
                Operator::NotRegexMatch => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.regex_match(op_span, &rhs, true, expr.span)
                }
                Operator::Modulo => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.modulo(op_span, &rhs, expr.span)
                }
                Operator::Pow => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.pow(op_span, &rhs, expr.span)
                }
                Operator::StartsWith => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.starts_with(op_span, &rhs, expr.span)
                }
                Operator::EndsWith => {
                    let rhs = eval_expression(engine_state, stack, rhs)?;
                    lhs.ends_with(op_span, &rhs, expr.span)
                }
            }
        }
        Expr::Subexpression(block_id) => {
            let block = engine_state.get_block(*block_id);

            // FIXME: protect this collect with ctrl-c
            Ok(
                eval_subexpression(engine_state, stack, block, PipelineData::new())?
                    .into_value(expr.span),
            )
        }
        Expr::RowCondition(block_id) | Expr::Block(block_id) => {
            let mut captures = HashMap::new();
            let block = engine_state.get_block(*block_id);

            for var_id in &block.captures {
                captures.insert(*var_id, stack.get_var(*var_id, expr.span)?);
            }
            Ok(Value::Block {
                val: *block_id,
                captures,
            })
        }
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(engine_state, stack, expr)?);
            }
            Ok(Value::List(output))
        }
        Expr::Record(fields) => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (col, val) in fields {
                // avoid duplicate cols.
                let col_name = eval_expression(engine_state, stack, col)?.as_string(expr.span)?;
                let pos = cols.iter().position(|c| c == &col_name);
                match pos {
                    Some(index) => {
                        vals[index] = eval_expression(engine_state, stack, val)?;
                    }
                    None => {
                        cols.push(col_name);
                        vals.push(eval_expression(engine_state, stack, val)?);
                    }
                }
            }

            Ok(Value::Record { cols, vals })
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers
                    .push(eval_expression(engine_state, stack, expr)?.as_string(expr.span)?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_expression(engine_state, stack, expr)?);
                }
                output_rows.push(Value::Record {
                    cols: output_headers.clone(),
                    vals: row,
                });
            }
            Ok(Value::List(output_rows))
        }
        Expr::Keyword(_, _, expr) => eval_expression(engine_state, stack, expr),
        Expr::StringInterpolation(exprs) => {
            let mut parts = vec![];
            for expr in exprs {
                parts.push(eval_expression(engine_state, stack, expr)?);
            }

            let config = engine_state.get_config();

            parts
                .into_iter()
                .into_pipeline_data(None)
                .collect_string("", config, expr.span)
                .map(|x| Value::String(x))
        }
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Filepath(s) => {
            let cwd = current_dir_str(engine_state, stack, expr.span)?;
            let path = expand_path_with(s, cwd);

            Ok(Value::String(path.to_string_lossy().to_string()))
        }
        Expr::Directory(s) => {
            if s == "-" {
                Ok(Value::String("-".to_string()))
            } else {
                let cwd = current_dir_str(engine_state, stack, expr.span)?;
                let path = expand_path_with(s, cwd);

                Ok(Value::String(path.to_string_lossy().to_string()))
            }
        }
        Expr::GlobPattern(s) => {
            let cwd = current_dir_str(engine_state, stack, expr.span)?;
            let path = expand_path_with(s, cwd);

            Ok(Value::String(path.to_string_lossy().to_string()))
        }
        Expr::Signature(_) => Ok(Value::Nothing),
        Expr::Garbage => Ok(Value::Nothing),
        Expr::Nothing => Ok(Value::Nothing),
    }
}

/// Checks the expression to see if it's a internal or external call. If so, passes the input
/// into the call and gets out the result
/// Otherwise, invokes the expression
pub fn eval_expression_with_input(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    match expr {
        Expression {
            expr: Expr::Call(call),
            ..
        } => {
            if !redirect_stdout || redirect_stderr {
                // we're doing something different than the defaults
                let mut call = call.clone();
                call.redirect_stdout = redirect_stdout;
                call.redirect_stderr = redirect_stderr;
                input = eval_call(engine_state, stack, &call, input)?;
            } else {
                input = eval_call(engine_state, stack, call, input)?;
            }
        }
        Expression {
            expr: Expr::ExternalCall(head, args),
            ..
        } => {
            input = eval_external(
                engine_state,
                stack,
                head,
                args,
                input,
                redirect_stdout,
                redirect_stderr,
            )?;
        }

        Expression {
            expr: Expr::Subexpression(block_id),
            ..
        } => {
            let block = engine_state.get_block(*block_id);

            // FIXME: protect this collect with ctrl-c
            input = eval_subexpression(engine_state, stack, block, input)?;
        }

        elem => {
            input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
        }
    }

    Ok(input)
}

pub fn eval_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    let num_pipelines = block.len();
    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        for (i, elem) in pipeline.expressions.iter().enumerate() {
            input = eval_expression_with_input(
                engine_state,
                stack,
                elem,
                input,
                redirect_stdout || (i != pipeline.expressions.len() - 1),
                redirect_stderr,
            )?
        }

        if pipeline_idx < (num_pipelines) - 1 {
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {}
                PipelineData::ExternalStream {
                    ref mut exit_code, ..
                } => {
                    let exit_code = exit_code.take();

                    // Drain the input to the screen via tabular output
                    let config = engine_state.get_config();

                    match engine_state.find_decl("table".as_bytes(), &[]) {
                        Some(decl_id) => {
                            let table = engine_state.get_decl(decl_id).run(
                                engine_state,
                                stack,
                                &Call::new(Span::new(0, 0)),
                                input,
                            )?;

                            for item in table {
                                if let Value::Error(error) = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", config);
                                out.push('\n');

                                stdout_write_all_and_flush(out)?
                            }
                        }
                        None => {
                            for item in input {
                                if let Value::Error(error) = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", config);
                                out.push('\n');

                                stdout_write_all_and_flush(out)?
                            }
                        }
                    };

                    if let Some(exit_code) = exit_code {
                        let mut v: Vec<_> = exit_code.collect();

                        if let Some(v) = v.pop() {
                            stack.add_env_var("LAST_EXIT_CODE".into(), v);
                        }
                    }
                }
                _ => {
                    // Drain the input to the screen via tabular output
                    let config = engine_state.get_config();

                    match engine_state.find_decl("table".as_bytes(), &[]) {
                        Some(decl_id) => {
                            let table = engine_state.get_decl(decl_id).run(
                                engine_state,
                                stack,
                                &Call::new(Span::new(0, 0)),
                                input,
                            )?;

                            for item in table {
                                if let Value::Error(error) = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", config);
                                out.push('\n');

                                stdout_write_all_and_flush(out)?
                            }
                        }
                        None => {
                            for item in input {
                                if let Value::Error(error) = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", config);
                                out.push('\n');

                                stdout_write_all_and_flush(out)?
                            }
                        }
                    };
                }
            }

            input = PipelineData::new()
        }
    }

    Ok(input)
}

pub fn eval_subexpression(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
) -> Result<PipelineData, ShellError> {
    for pipeline in block.pipelines.iter() {
        for expr in pipeline.expressions.iter() {
            input = eval_expression_with_input(engine_state, stack, expr, input, true, false)?
        }
    }

    Ok(input)
}

fn extract_custom_completion_from_arg(engine_state: &EngineState, shape: &SyntaxShape) -> String {
    return match shape {
        SyntaxShape::Custom(_, custom_completion_decl_id) => {
            let custom_completion_command = engine_state.get_decl(*custom_completion_decl_id);
            let custom_completion_command_name: &str = &*custom_completion_command.name();
            custom_completion_command_name.to_string()
        }
        _ => "".to_string(),
    };
}

pub fn create_scope(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<Value, ShellError> {
    let mut output_cols = vec![];
    let mut output_vals = vec![];

    let mut vars = vec![];
    let mut commands = vec![];
    let mut aliases = vec![];
    let mut modules = vec![];

    let mut vars_map = HashMap::new();
    let mut commands_map = HashMap::new();
    let mut aliases_map = HashMap::new();
    let mut modules_map = HashMap::new();
    let mut visibility = Visibility::new();

    for overlay_frame in engine_state.active_overlays(&[]) {
        vars_map.extend(&overlay_frame.vars);
        commands_map.extend(&overlay_frame.decls);
        aliases_map.extend(&overlay_frame.aliases);
        modules_map.extend(&overlay_frame.modules);

        visibility.merge_with(overlay_frame.visibility.clone());
    }

    for var in vars_map {
        let var_name = Value::String(String::from_utf8_lossy(var.0).to_string());

        let var_type = Value::String(engine_state.get_var(*var.1).ty.to_string());

        let var_value = if let Ok(val) = stack.get_var(*var.1, span) {
            val
        } else {
            Value::Nothing
        };

        vars.push(Value::Record {
            cols: vec!["name".to_string(), "type".to_string(), "value".to_string()],
            vals: vec![var_name, var_type, var_value],
        })
    }

    for (command_name, decl_id) in commands_map {
        if visibility.is_decl_id_visible(decl_id) {
            let mut cols = vec![];
            let mut vals = vec![];

            let mut module_commands = vec![];
            for module in &modules_map {
                let module_name = String::from_utf8_lossy(module.0).to_string();
                let module_id = engine_state.find_module(module.0, &[]);
                if let Some(module_id) = module_id {
                    let module = engine_state.get_module(module_id);
                    if module.has_decl(command_name) {
                        module_commands.push(module_name);
                    }
                }
            }

            cols.push("command".into());
            vals.push(Value::String(
                String::from_utf8_lossy(command_name).to_string(),
            ));

            cols.push("module_name".into());
            vals.push(Value::String(module_commands.join(", ")));

            let decl = engine_state.get_decl(*decl_id);
            let signature = decl.signature();

            cols.push("category".to_string());
            vals.push(Value::String(signature.category.to_string()));

            // signature
            let mut sig_records = vec![];
            {
                let sig_cols = vec![
                    "command".to_string(),
                    "parameter_name".to_string(),
                    "parameter_type".to_string(),
                    "syntax_shape".to_string(),
                    "is_optional".to_string(),
                    "short_flag".to_string(),
                    "description".to_string(),
                    "custom_completion".to_string(),
                ];

                // required_positional
                for req in signature.required_positional {
                    let sig_vals = vec![
                        Value::String(signature.name.to_string()),
                        Value::String(req.name),
                        Value::String("positional".to_string()),
                        Value::String(req.shape.to_string()),
                        Value::Bool(false),
                        Value::Nothing,
                        Value::String(req.desc),
                        Value::String(extract_custom_completion_from_arg(engine_state, &req.shape)),
                    ];

                    sig_records.push(Value::Record {
                        cols: sig_cols.clone(),
                        vals: sig_vals,
                    });
                }

                // optional_positional
                for opt in signature.optional_positional {
                    let sig_vals = vec![
                        Value::String(signature.name.to_string()),
                        Value::String(opt.name),
                        Value::String("positional".to_string()),
                        Value::String(opt.shape.to_string()),
                        Value::Bool(true),
                        Value::Nothing,
                        Value::String(opt.desc),
                        Value::String(extract_custom_completion_from_arg(engine_state, &opt.shape)),
                    ];

                    sig_records.push(Value::Record {
                        cols: sig_cols.clone(),
                        vals: sig_vals,
                    });
                }

                {
                    // rest_positional
                    if let Some(rest) = signature.rest_positional {
                        let sig_vals = vec![
                            Value::String(signature.name.to_string()),
                            Value::String(rest.name),
                            Value::String("rest".to_string()),
                            Value::String(rest.shape.to_string()),
                            Value::Bool(true),
                            Value::Nothing,
                            Value::String(rest.desc),
                            Value::String(extract_custom_completion_from_arg(
                                engine_state,
                                &rest.shape,
                            )),
                        ];

                        sig_records.push(Value::Record {
                            cols: sig_cols.clone(),
                            vals: sig_vals,
                        });
                    }
                }

                // named flags
                for named in signature.named {
                    let flag_type;

                    // Skip the help flag
                    if named.long == "help" {
                        continue;
                    }

                    let mut custom_completion_command_name: String = "".to_string();
                    let shape = if let Some(arg) = named.arg {
                        flag_type = Value::String("named".to_string());
                        custom_completion_command_name =
                            extract_custom_completion_from_arg(engine_state, &arg);
                        Value::String(arg.to_string())
                    } else {
                        flag_type = Value::String("switch".to_string());
                        Value::Nothing
                    };

                    let short_flag = if let Some(c) = named.short {
                        Value::String(c.to_string())
                    } else {
                        Value::Nothing
                    };

                    let sig_vals = vec![
                        Value::String(signature.name.to_string()),
                        Value::String(named.long),
                        flag_type,
                        shape,
                        Value::Bool(!named.required),
                        short_flag,
                        Value::String(named.desc),
                        Value::String(custom_completion_command_name),
                    ];

                    sig_records.push(Value::Record {
                        cols: sig_cols.clone(),
                        vals: sig_vals,
                    });
                }
            }

            cols.push("signature".to_string());
            vals.push(Value::List(sig_records));

            cols.push("usage".to_string());
            vals.push(Value::String(decl.usage().into()));

            cols.push("examples".to_string());
            vals.push(Value::List(
                decl.examples()
                    .into_iter()
                    .map(|x| Value::Record {
                        cols: vec!["description".into(), "example".into()],
                        vals: vec![
                            Value::String(x.description.to_string()),
                            Value::String(x.example.to_string()),
                        ],
                    })
                    .collect(),
            ));

            cols.push("is_binary".to_string());
            vals.push(Value::Bool(decl.is_binary()));

            cols.push("is_builtin".to_string());
            // we can only be a is_builtin or is_custom, not both
            vals.push(Value::Bool(decl.get_block_id().is_none()));

            cols.push("is_sub".to_string());
            vals.push(Value::Bool(decl.is_sub()));

            cols.push("is_plugin".to_string());
            vals.push(Value::Bool(decl.is_plugin().is_some()));

            cols.push("is_custom".to_string());
            vals.push(Value::Bool(decl.get_block_id().is_some()));

            cols.push("is_keyword".into());
            vals.push(Value::Bool(decl.is_parser_keyword()));

            cols.push("is_extern".to_string());
            vals.push(Value::Bool(decl.is_known_external()));

            cols.push("creates_scope".to_string());
            vals.push(Value::Bool(signature.creates_scope));

            cols.push("extra_usage".to_string());
            vals.push(Value::String(decl.extra_usage().into()));

            let search_terms = decl.search_terms();
            cols.push("search_terms".to_string());
            vals.push(if search_terms.is_empty() {
                Value::Nothing
            } else {
                Value::String(search_terms.join(", "))
            });

            commands.push(Value::Record { cols, vals })
        }
    }

    for (alias_name, alias_id) in aliases_map {
        if visibility.is_alias_id_visible(alias_id) {
            let alias = engine_state.get_alias(*alias_id);
            let mut alias_text = String::new();
            for span in alias {
                let contents = engine_state.get_span_contents(span);
                if !alias_text.is_empty() {
                    alias_text.push(' ');
                }
                alias_text.push_str(&String::from_utf8_lossy(contents));
            }
            aliases.push((
                Value::String(String::from_utf8_lossy(alias_name).to_string()),
                Value::String(alias_text),
            ));
        }
    }

    for module in modules_map {
        modules.push(Value::String(String::from_utf8_lossy(module.0).to_string()));
    }

    output_cols.push("vars".to_string());
    output_vals.push(Value::List(vars));

    commands.sort_by(|a, b| match (a, b) {
        (Value::Record { vals: rec_a, .. }, Value::Record { vals: rec_b, .. }) => {
            // Comparing the first value from the record
            // It is expected that the first value is the name of the column
            // The names of the commands should be a value string
            match (rec_a.get(0), rec_b.get(0)) {
                (Some(val_a), Some(val_b)) => match (val_a, val_b) {
                    (Value::String(str_a), Value::String(str_b)) => str_a.cmp(str_b),
                    _ => Ordering::Equal,
                },
                _ => Ordering::Equal,
            }
        }
        _ => Ordering::Equal,
    });
    output_cols.push("commands".to_string());
    output_vals.push(Value::List(commands));

    aliases.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    output_cols.push("aliases".to_string());
    output_vals.push(Value::List(
        aliases
            .into_iter()
            .map(|(alias, value)| Value::Record {
                cols: vec!["alias".into(), "expansion".into()],
                vals: vec![alias, value],
            })
            .collect(),
    ));

    modules.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    output_cols.push("modules".to_string());
    output_vals.push(Value::List(modules));

    Ok(Value::Record {
        cols: output_cols,
        vals: output_vals,
    })
}

pub fn eval_variable(
    engine_state: &EngineState,
    stack: &Stack,
    var_id: VarId,
    span: Span,
) -> Result<Value, ShellError> {
    match var_id {
        nu_protocol::NU_VARIABLE_ID => {
            // $nu
            let mut output_cols = vec![];
            let mut output_vals = vec![];

            if let Some(mut config_path) = nu_path::config_dir() {
                config_path.push("nushell");
                let mut env_config_path = config_path.clone();

                let mut history_path = config_path.clone();

                history_path.push("history.txt");

                output_cols.push("history-path".into());
                output_vals.push(Value::String(history_path.to_string_lossy().to_string()));

                config_path.push("config.nu");

                output_cols.push("config-path".into());
                output_vals.push(Value::String(config_path.to_string_lossy().to_string()));

                env_config_path.push("env.nu");

                output_cols.push("env-path".into());
                output_vals.push(Value::String(env_config_path.to_string_lossy().to_string()));
            }

            #[cfg(feature = "plugin")]
            if let Some(path) = &engine_state.plugin_signatures {
                if let Some(path_str) = path.to_str() {
                    output_cols.push("plugin-path".into());
                    output_vals.push(Value::String(path_str.into()));
                }
            }

            output_cols.push("scope".into());
            output_vals.push(create_scope(engine_state, stack, span)?);

            if let Some(home_path) = nu_path::home_dir() {
                if let Some(home_path_str) = home_path.to_str() {
                    output_cols.push("home-path".into());
                    output_vals.push(Value::String(home_path_str.into()))
                }
            }

            let temp = std::env::temp_dir();
            if let Some(temp_path) = temp.to_str() {
                output_cols.push("temp-path".into());
                output_vals.push(Value::String(temp_path.into()))
            }

            let pid = std::process::id();
            output_cols.push("pid".into());
            output_vals.push(Value::Int(pid as i64));

            let sys = sysinfo::System::new();
            let ver = match sys.kernel_version() {
                Some(v) => v,
                None => "unknown".into(),
            };

            let os_record = Value::Record {
                cols: vec![
                    "name".into(),
                    "arch".into(),
                    "family".into(),
                    "kernel_version".into(),
                ],
                vals: vec![
                    Value::String(std::env::consts::OS.to_string()),
                    Value::String(std::env::consts::ARCH.to_string()),
                    Value::String(std::env::consts::FAMILY.to_string()),
                    Value::String(ver),
                ],
            };
            output_cols.push("os-info".into());
            output_vals.push(os_record);

            Ok(Value::Record {
                cols: output_cols,
                vals: output_vals,
            })
        }
        ENV_VARIABLE_ID => {
            let env_vars = stack.get_env_vars(engine_state);
            let env_columns = env_vars.keys();
            let env_values = env_vars.values();

            let mut pairs = env_columns
                .map(|x| x.to_string())
                .zip(env_values.cloned())
                .collect::<Vec<(String, Value)>>();

            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            let (env_columns, env_values) = pairs.into_iter().unzip();

            Ok(Value::Record {
                cols: env_columns,
                vals: env_values,
            })
        }
        var_id => stack.get_var(var_id, span),
    }
}

fn compute(size: i64, unit: Unit, span: Span) -> Value {
    match unit {
        Unit::Byte => Value::Filesize(size),
        Unit::Kilobyte => Value::Filesize(size * 1000),
        Unit::Megabyte => Value::Filesize(size * 1000 * 1000),
        Unit::Gigabyte => Value::Filesize(size * 1000 * 1000 * 1000),
        Unit::Terabyte => Value::Filesize(size * 1000 * 1000 * 1000 * 1000),
        Unit::Petabyte => Value::Filesize(size * 1000 * 1000 * 1000 * 1000 * 1000),

        Unit::Kibibyte => Value::Filesize(size * 1024),
        Unit::Mebibyte => Value::Filesize(size * 1024 * 1024),
        Unit::Gibibyte => Value::Filesize(size * 1024 * 1024 * 1024),
        Unit::Tebibyte => Value::Filesize(size * 1024 * 1024 * 1024 * 1024),
        Unit::Pebibyte => Value::Filesize(size * 1024 * 1024 * 1024 * 1024 * 1024),

        Unit::Nanosecond => Value::Duration(size),
        Unit::Microsecond => Value::Duration(size * 1000),
        Unit::Millisecond => Value::Duration(size * 1000 * 1000),
        Unit::Second => Value::Duration(size * 1000 * 1000 * 1000),
        Unit::Minute => Value::Duration(size * 1000 * 1000 * 1000 * 60),
        Unit::Hour => Value::Duration(size * 1000 * 1000 * 1000 * 60 * 60),
        Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
            Some(val) => Value::Duration(val),
            None => Value::Error(ShellError::GenericError(
                "duration too large".into(),
                "duration too large".into(),
                Some(span),
                None,
                Vec::new(),
            )),
        },
        Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
            Some(val) => Value::Duration(val),
            None => Value::Error(ShellError::GenericError(
                "duration too large".into(),
                "duration too large".into(),
                Some(span),
                None,
                Vec::new(),
            )),
        },
    }
}
