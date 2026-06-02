use nu_engine::{
    CallEval, command_prelude::*, get_eval_block_with_early_return, get_eval_expression,
};
use nu_path::{absolute_with, is_windows_device_path};
use nu_protocol::{BlockId, Value, engine::CommandType, shell_error::io::IoError};
use std::sync::Arc;

/// Run a script file in an isolated scope as part of a pipeline.
#[derive(Clone)]
pub struct Run;

impl Command for Run {
    fn name(&self) -> &str {
        "run"
    }

    fn signature(&self) -> Signature {
        Signature::build("run")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "filename",
                SyntaxShape::OneOf(vec![SyntaxShape::Filepath, SyntaxShape::Nothing]),
                "The filepath to the script file to run (`null` for no-op).",
            )
            .rest(
                "arguments",
                SyntaxShape::Any,
                "Arguments to pass to the script's `def main` if it exists.",
            )
            .allows_unknown_args()
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Runs a script file in an isolated scope as part of a pipeline."
    }

    fn extra_description(&self) -> &str {
        "This command is a parser keyword. For details, check:
   https://www.nushell.sh/book/thinking_in_nu.html"
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // `run null` is parsed as a no-op so pipelines can keep flowing without
        // introducing conditional command dispatch in the runtime path.
        if call.get_parser_info(stack, "noop").is_some() {
            return Ok(input);
        }

        // Parser-time metadata tells us exactly which script block was compiled for this call.
        // We intentionally execute that precompiled block instead of reparsing at runtime.
        //
        // - `block_id`: block compiled from the resolved script file
        // - `block_id_name`: script file path string captured at parse time
        let block_id: i64 = call.req_parser_info(engine_state, stack, "block_id")?;
        let block_id_name: String = call.req_parser_info(engine_state, stack, "block_id_name")?;
        let block_id = BlockId::new(block_id as usize);
        let block = engine_state.get_block(block_id).clone();

        // Resolve the script path to an absolute path for consistent `CURRENT_FILE` / `FILE_PWD`
        // behavior. Device paths on Windows are already absolute-like and must be preserved.
        let cwd = engine_state.cwd_as_string(Some(stack))?;
        let pb = std::path::PathBuf::from(block_id_name);
        let parent = pb.parent().unwrap_or(std::path::Path::new(""));
        let file_path = if is_windows_device_path(pb.as_path()) {
            pb.clone()
        } else {
            let path = absolute_with(pb.as_path(), cwd)
                .map_err(|err| IoError::new(err, call.head, pb.clone()))?;
            match path.try_exists() {
                Ok(true) => {}
                Ok(false) => {
                    return Err(IoError::new(ErrorKind::FileNotFound, call.head, pb.clone()).into());
                }
                Err(e) => return Err(IoError::new(e, call.head, pb.clone()).into()),
            };
            path
        };

        // Stash caller values so we can restore them after execution. `run` should expose file
        // context to the script, but must not leak modified values back to the caller.
        let old_file_pwd = stack.get_env_var(engine_state, "FILE_PWD").cloned();
        let old_current_file = stack.get_env_var(engine_state, "CURRENT_FILE").cloned();

        // Mirror `source`-style file context for script execution.
        stack.add_env_var(
            "FILE_PWD".to_string(),
            Value::string(parent.to_string_lossy(), call.head),
        );
        stack.add_env_var(
            "CURRENT_FILE".to_string(),
            Value::string(file_path.to_string_lossy(), call.head),
        );

        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
        let return_result = (|| {
            // If parser metadata includes a `main` entrypoint, invoke that specific declaration.
            // Otherwise evaluate the full script block as a pipeline transform.
            if call.get_parser_info(stack, "main_block_id").is_some() {
                // These IDs are parser-scoped to the resolved script and avoid cross-script `main`
                // lookup collisions from ambient declarations.
                let main_block_id: i64 =
                    call.req_parser_info(engine_state, stack, "main_block_id")?;
                let main_block = engine_state
                    .get_block(BlockId::new(main_block_id as usize))
                    .clone();
                let signature = (*main_block.signature).clone();
                let callee_stack = stack.gather_captures(engine_state, &main_block.captures);
                let mut call_eval = CallEval::new(
                    callee_stack,
                    call.head,
                    main_block.span.unwrap_or(call.head),
                    eval_block_with_early_return,
                );

                // Forward remaining run arguments (`run file.nu ...args`) to `main`.
                // This helper normalizes long/short flags and supports AST+IR call representations
                // while delegating actual binding/type validation to CallEval.
                bind_main_arguments(engine_state, stack, call, &signature, &mut call_eval)?;
                call_eval.finalize_for_signature(&signature)?;

                // Execute a signature-stripped copy of `main` after manually binding all
                // arguments so pipeline input remains available as `$in` and is not rebound
                // to positional parameters by call-time argument machinery.
                // Pipeline input passes through as `$in`; positional args come only from
                // explicit `run file.nu ...args` tokens bound above.
                let mut executable_main_block = (*main_block).clone();
                *executable_main_block.signature = Signature::new("main");

                call_eval.run_prebound(engine_state, &executable_main_block, input)
            } else {
                // No explicit `main`: execute the script block directly in an isolated child stack.
                // Parent scope values remain readable via stack parenting, but script mutations do
                // not leak back to the caller.
                let parent_stack = Arc::new(stack.clone());
                let mut callee_stack = Stack::with_parent(parent_stack);
                eval_block_with_early_return(engine_state, &mut callee_stack, &block, input)
                    .map(|p| p.body)
            }
        })();

        // Always restore caller file-context env after script evaluation (success or error).
        // If values did not exist before `run`, remove them instead of leaving command-introduced
        // entries behind.
        if let Some(old_file_pwd) = old_file_pwd {
            stack.add_env_var("FILE_PWD".to_string(), old_file_pwd);
        } else {
            stack.remove_env_var(engine_state, "FILE_PWD");
        }
        if let Some(old_current_file) = old_current_file {
            stack.add_env_var("CURRENT_FILE".to_string(), old_current_file);
        } else {
            stack.remove_env_var(engine_state, "CURRENT_FILE");
        }

        return_result
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Run a simple transformation script in a pipeline.",
                example: r#""hello" | run transform.nu"#,
                result: None,
            },
            Example {
                description: "Run a script with arguments.",
                example: r#""test" | run format.nu --prefix ">>>" "#,
                result: None,
            },
            Example {
                description: "Run a script as part of a larger pipeline.",
                example: "ls | run process.nu | select name size",
                result: None,
            },
        ]
    }
}

/// Parse a source token that looks like a long or short named flag.
///
/// Returns `(long_name, short_name)` where:
/// - `--char` becomes `("char", None)`
/// - `-c` becomes `("c", Some("c"))`
fn parse_flag_name(token: &str) -> Option<(String, Option<String>)> {
    if let Some(flag_name) = token.strip_prefix("--")
        && !flag_name.is_empty()
    {
        return Some((flag_name.to_string(), None));
    }

    let mut chars = token.chars();
    if chars.next() == Some('-')
        && let Some(short) = chars.next()
        && chars.next().is_none()
        && short.is_ascii_alphabetic()
    {
        let short = short.to_string();
        return Some((short.clone(), Some(short)));
    }

    None
}

/// Parse a forwarded argument value into a flag token.
///
/// Source text is preferred so quoted literals like `"-c"` stay positional values.
fn parse_flag_token(engine_state: &EngineState, value: &Value) -> Option<(String, Option<String>)> {
    let span = value.span();
    let span_contents = engine_state.get_span_contents(span);
    if let Ok(token) = std::str::from_utf8(span_contents) {
        if let Some(flag) = parse_flag_name(token) {
            return Some(flag);
        }

        if token.starts_with('"') || token.starts_with('\'') {
            return None;
        }
    }

    match value {
        Value::String { val, .. } => parse_flag_name(val),
        _ => None,
    }
}

/// Check whether a parsed flag token matches a named parameter from a signature.
///
/// Matches on the long name (`--char` → `"char"`) or by comparing the single short character
/// extracted from a `-c` token against the flag's declared short character.
fn matches_named_flag(named: &Flag, long: &str, short: Option<&str>) -> bool {
    named.long == long || short.and_then(|name| name.chars().next()) == named.short
}

/// Resolve a parsed flag token to the matching signature flag, if any.
fn resolve_named_flag<'a>(
    signature: &'a Signature,
    long: &str,
    short: Option<&str>,
) -> Option<&'a Flag> {
    signature
        .named
        .iter()
        .find(|named| matches_named_flag(named, long, short))
}

/// Bind explicit `run file.nu ...args` arguments onto a script `def main` call evaluator.
fn bind_main_arguments(
    engine_state: &EngineState,
    caller_stack: &mut Stack,
    call: &Call,
    signature: &Signature,
    call_eval: &mut CallEval,
) -> Result<(), ShellError> {
    let rest_values = collect_explicit_run_arguments(engine_state, caller_stack, call)?;

    let mut index = 0;
    while index < rest_values.len() {
        if let Some((long, short)) = parse_flag_token(engine_state, &rest_values[index]) {
            let matched_flag = resolve_named_flag(signature, &long, short.as_deref());
            if let Some(flag) = matched_flag {
                let expects_value = flag.arg.is_some();
                let value = if expects_value
                    && index + 1 < rest_values.len()
                    && parse_flag_token(engine_state, &rest_values[index + 1]).is_none()
                {
                    index += 1;
                    Some(std::borrow::Cow::Owned(rest_values[index].clone()))
                } else {
                    None
                };

                call_eval.add_named(signature, &flag.long, short, value)?;
            }
        } else {
            call_eval.add_positional(
                signature,
                std::borrow::Cow::Owned(rest_values[index].clone()),
            )?;
        }

        index += 1;
    }

    Ok(())
}

/// Collect only the explicit run arguments after the script filename.
fn collect_explicit_run_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Vec<Value>, ShellError> {
    let eval_expression = get_eval_expression(engine_state);
    call.rest_iter_flattened(engine_state, stack, eval_expression, 1)
}
