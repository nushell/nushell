use super::deferred_arguments::{
    ArgumentTarget, DeferredArgument, bind_deferred_arguments, call_arguments, deferred_arguments,
    split_off_callee_arguments,
};
use nu_engine::{CallEval, command_prelude::*, get_eval_block_with_early_return, get_full_help};
use nu_parser::{find_main_block_id_in_script, parse};
use nu_path::{absolute_with, is_windows_device_path};
use nu_protocol::{
    BlockId, CompareTypes, DeclId, Flag, Value,
    ast::Block,
    engine::{self, CommandType, StateWorkingSet, named_flags::list_spread_before_required_error},
    parser_path::{MAX_RUN_SCRIPT_BYTES, ScriptLoadError, read_run_script_file},
    shell_error::{generic::GenericError, io::IoError},
};
use std::{borrow::Cow, sync::Arc};

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
            // The parser appends the script's arguments itself, and passes each spread after the
            // path (`run x.nu ...$args`) as a list spread. The evaluator checks it against `run`'s
            // own signature, which needs a rest parameter to let it through to `main`.
            .rest(
                "arguments",
                SyntaxShape::Any,
                "Arguments to pass to the script's `def main` if it exists.",
            )
            .switch(
                "full-reparse",
                "Reload and reparse the script on every invocation instead of using parser-cached blocks.",
                Some('f'),
            )
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
        let block_id_name: String = call.req_parser_info(engine_state, stack, "block_id_name")?;
        let args = run_arguments(engine_state, stack, call)?;
        let full_reparse =
            call.get_parser_info(stack, "full_reparse").is_some() || args.full_reparse;

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

        let mut full_reparse_engine_state = None;
        let (block, main_block_id) = if full_reparse {
            let (reparsed_engine_state, reparsed_block, reparsed_main_block_id) =
                parse_run_script_fresh(engine_state, &file_path, call.head)?;
            full_reparse_engine_state = Some(reparsed_engine_state);
            (reparsed_block, reparsed_main_block_id)
        } else {
            // - `block_id`: block compiled from the resolved script file
            // - `main_block_id`: the block of the script's `def main`, when it has one
            let block_id: i64 = call.req_parser_info(engine_state, stack, "block_id")?;
            let block = engine_state
                .get_block(BlockId::new(block_id as usize))
                .clone();
            let main_block_id = if call.get_parser_info(stack, "main_block_id").is_some() {
                let main_block_id: i64 =
                    call.req_parser_info(engine_state, stack, "main_block_id")?;
                Some(BlockId::new(main_block_id as usize))
            } else {
                None
            };
            (block, main_block_id)
        };
        let eval_engine_state = full_reparse_engine_state.as_ref().unwrap_or(engine_state);

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

        let eval_block_with_early_return = get_eval_block_with_early_return(eval_engine_state);
        let return_result = (|| {
            // If the script has a `main` entrypoint, invoke that specific declaration.
            // Otherwise evaluate the full script block as a pipeline transform.
            if let Some(main_block_id) = main_block_id {
                let main_block = eval_engine_state.get_block(main_block_id);
                let signature = &main_block.signature;
                let mut callee_stack =
                    stack.gather_captures(eval_engine_state, &main_block.captures);
                // Like a call to a custom command, a call to `main` counts toward the recursion
                // limit, so a script that runs itself fails instead of overflowing the stack.
                callee_stack.recursion_count += 1;
                // As for a call to `main`, an error for a missing argument points at the call, and
                // an optional argument that's left out is a `null` with the call's span.
                let mut call_eval = CallEval::new(
                    callee_stack,
                    call.head,
                    call.head,
                    eval_block_with_early_return,
                );

                // Forward the arguments after the script path (`run file.nu ...args`) to `main`.
                let mut main_arguments = MainArguments {
                    engine_state,
                    call_eval: &mut call_eval,
                };
                if bind_deferred_arguments(engine_state, signature, args.main, &mut main_arguments)?
                {
                    // The help shows `main`'s examples, which are on its declaration.
                    let main_decl = (0..eval_engine_state.num_decls())
                        .rev()
                        .map(|decl_id| eval_engine_state.get_decl(DeclId::new(decl_id)))
                        .find(|decl| decl.block_id() == Some(main_block_id))
                        .ok_or_else(|| ShellError::NushellFailedSpanned {
                            msg: "`run` couldn't find the declaration of the script's `main`"
                                .into(),
                            label: "for this script".into(),
                            span: call.head,
                        })?;
                    let help = get_full_help(main_decl, eval_engine_state, stack, call.head);
                    return Ok(Value::string(help, call.head).into_pipeline_data());
                }

                // No parameter takes the pipeline input, so it reaches `main` as `$in`.
                call_eval.run(eval_engine_state, main_block, input)
            } else {
                // No explicit `main`: execute the script block directly in an isolated child stack.
                // Parent scope values remain readable via stack parenting, but script mutations do
                // not leak back to the caller.
                let parent_stack = Arc::new(stack.clone());
                let mut callee_stack = Stack::with_parent(parent_stack);
                // The script body counts toward the recursion limit, as `main` does.
                callee_stack.recursion_count += 1;
                eval_block_with_early_return(eval_engine_state, &mut callee_stack, &block, input)
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
            Example {
                description: "Always reload and reparse a script before each invocation.",
                example: "watch . -g *.nu | each -f { run --full-reparse ./test.nu }",
                result: None,
            },
        ]
    }
}

/// Reload, reparse, and compile a script file against a cloned engine state.
///
/// This is used by `run --full-reparse` to bypass parser-time script caching while keeping
/// declaration resolution and execution isolated from the caller's engine state.
///
/// Parse errors are surfaced at runtime as `ShellError::Generic`, which is an intentional behavior
/// difference from parse-time `run` compilation.
fn parse_run_script_fresh(
    engine_state: &EngineState,
    file_path: &std::path::Path,
    call_head: Span,
) -> Result<(EngineState, Arc<Block>, Option<BlockId>), ShellError> {
    let display_path = file_path.display().to_string();
    let contents = match read_run_script_file(file_path, MAX_RUN_SCRIPT_BYTES) {
        Ok(contents) => contents,
        Err(ScriptLoadError::TooLarge { size, max_size }) => {
            return Err(GenericError::new(
                "Script file is too large to load",
                format!(
                    "Refusing to load files larger than {max_size} bytes for `run` (file is {size} bytes): {display_path}"
                ),
                call_head,
            )
            .into());
        }
        Err(ScriptLoadError::NotText) => {
            return Err(GenericError::new(
                "Script file does not appear to be text",
                format!(
                    "The file does not look like UTF-8 text and cannot be loaded by `run`: {display_path}"
                ),
                call_head,
            )
            .into());
        }
        Err(ScriptLoadError::Unreadable) => {
            return Err(GenericError::new(
                "Failed to read script",
                format!("Could not read script file: {display_path}"),
                call_head,
            )
            .into());
        }
    };

    let mut full_reparse_engine_state = engine_state.clone();
    let mut working_set = StateWorkingSet::new(&full_reparse_engine_state);
    working_set
        .files
        .push(file_path.to_path_buf(), call_head)
        .map_err(|err| GenericError::new("Failed to parse script", err.to_string(), call_head))?;

    let filename = file_path.to_string_lossy();
    let script_block = parse(&mut working_set, Some(filename.as_ref()), &contents, false);
    let script_main_block_id = find_main_block_id_in_script(&working_set, &script_block);
    working_set.files.pop();

    if let Some(parse_error) = working_set.parse_errors.first() {
        return Err(GenericError::new(
            "Failed to parse script",
            parse_error.to_string(),
            call_head,
        )
        .into());
    }

    let delta = working_set.render();
    full_reparse_engine_state.merge_delta(delta)?;

    Ok((
        full_reparse_engine_state,
        script_block,
        script_main_block_id,
    ))
}

/// The evaluated arguments of a `run` call.
struct RunArguments {
    /// Whether a record spread before the script path turns on `--full-reparse`, as in
    /// `run ...$opts x.nu`. The parser reads a `--full-reparse` written as a flag.
    full_reparse: bool,
    /// The arguments after the script path, for `main`.
    main: Vec<DeferredArgument>,
}

/// Evaluate the arguments of a `run` call.
fn run_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<RunArguments, ShellError> {
    let mut args = call_arguments(engine_state, stack, call, &Run)?;
    let main = deferred_arguments(split_off_callee_arguments(&mut args))?;
    Ok(RunArguments {
        full_reparse: spread_full_reparse(&args)?,
        main,
    })
}

/// Whether `run`'s own arguments, the ones up to the script path, turn on `--full-reparse`.
///
/// A record spread among them has been expanded into flags against `run`'s signature by now. A
/// spread that's left would fill rest arguments, which can't come before the path, as for any
/// command with a required positional argument.
fn spread_full_reparse(run_args: &[engine::Argument]) -> Result<bool, ShellError> {
    let mut full_reparse = false;
    for arg in run_args {
        match arg {
            engine::Argument::Flag { data, name, .. } => {
                full_reparse |= &data[*name] == b"full-reparse";
            }
            engine::Argument::Spread {
                vals: Value::Error { error, .. },
                ..
            } => return Err((**error).clone()),
            engine::Argument::Spread { span, .. } => {
                return Err(list_spread_before_required_error(*span));
            }
            _ => {}
        }
    }
    Ok(full_reparse)
}

/// Point a type error for an argument after the script path at the value, and explain it.
///
/// The parser parses these arguments without `main`'s signature, so a bare `23` is an `int` even
/// where `main` takes a `string`, which a call to `main` would convert. A variable, a
/// subexpression or a record has the same type in a call to `main`, so its error gets no hint.
fn with_conversion_hint(
    engine_state: &EngineState,
    err: ShellError,
    value_span: Span,
) -> ShellError {
    match err {
        ShellError::CantConvert {
            to_type,
            from_type,
            help: None,
            ..
        } => {
            let text = engine_state.get_span_contents(value_span);
            let help = (!matches!(text.first(), Some(b'$' | b'(' | b'{'))).then(|| {
                if to_type == "string" {
                    "`run` doesn't convert script arguments; quote the value to pass a string"
                        .to_owned()
                } else {
                    format!(
                        "`run` doesn't convert script arguments; pass a value of type `{to_type}`"
                    )
                }
            });
            ShellError::CantConvert {
                to_type,
                from_type,
                span: value_span,
                help,
            }
        }
        err => err,
    }
}

/// Binds the arguments after the script path to the variables of the script's `main`.
///
/// Unlike a call to `main`, values aren't converted to the parameter's type, so `--name 42` for a
/// `string` flag is an error that says to quote it.
struct MainArguments<'a> {
    engine_state: &'a EngineState,
    call_eval: &'a mut CallEval,
}

impl ArgumentTarget for MainArguments<'_> {
    fn add_positional(
        &mut self,
        signature: &Signature,
        value: Value,
        span: Span,
    ) -> Result<(), ShellError> {
        self.call_eval
            .add_positional(signature, Cow::Owned(value))
            .map_err(|err| with_conversion_hint(self.engine_state, err, span))?;
        Ok(())
    }

    fn add_spread(
        &mut self,
        signature: &Signature,
        value: Value,
        span: Span,
    ) -> Result<(), ShellError> {
        self.call_eval
            .add_spread(signature, value, span)
            .map_err(|err| with_conversion_hint(self.engine_state, err, span))?;
        Ok(())
    }

    fn add_named(
        &mut self,
        signature: &Signature,
        flag: &Flag,
        _span: Span,
        value: Option<(Value, Span)>,
    ) -> Result<(), ShellError> {
        if let Some((value, value_span)) = &value {
            let flag_type = flag.arg.as_ref().map_or(Type::Bool, SyntaxShape::to_type);
            if !value.is_subtype_of(&flag_type) {
                let err = ShellError::CantConvert {
                    to_type: flag_type.to_string(),
                    from_type: value.get_type().to_string(),
                    span: *value_span,
                    help: None,
                };
                return Err(with_conversion_hint(self.engine_state, err, *value_span));
            }
        }
        self.call_eval.add_named(
            signature,
            &flag.long,
            flag.short.map(String::from),
            value.map(|(value, _)| Cow::Owned(value)),
        )?;
        Ok(())
    }
}
