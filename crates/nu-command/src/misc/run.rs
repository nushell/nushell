use nu_engine::{
    CallEval, command_prelude::*, get_eval_block_with_early_return, get_eval_expression,
    get_full_help,
};
use nu_parser::{find_main_decl_id_in_script, parse};
use nu_path::{absolute_with, is_windows_device_path};
use nu_protocol::{
    BlockId, CompareTypes, DeclId, ParseError, Value,
    ast::{self, Block},
    did_you_mean,
    engine::{self, CallImpl, CommandType, StateWorkingSet, named_flags::can_rest_spread},
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
            // The parser appends the script's arguments itself, but the evaluator checks a list
            // spread (`run x.nu ...$list`) against `run`'s own signature, which needs a rest
            // parameter to let it through to `main`.
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
        let full_reparse = call.get_parser_info(stack, "full_reparse").is_some();

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
        let (block, main_decl_id) = if full_reparse {
            let (reparsed_engine_state, reparsed_block, reparsed_main_decl_id) =
                parse_run_script_fresh(engine_state, &file_path, call.head)?;
            full_reparse_engine_state = Some(reparsed_engine_state);
            (reparsed_block, reparsed_main_decl_id)
        } else {
            // - `block_id`: block compiled from the resolved script file
            // - `main_decl_id`: the script's `def main`, when it has one
            let block_id: i64 = call.req_parser_info(engine_state, stack, "block_id")?;
            let block = engine_state
                .get_block(BlockId::new(block_id as usize))
                .clone();
            let main_decl_id = if call.get_parser_info(stack, "main_decl_id").is_some() {
                let main_decl_id: i64 =
                    call.req_parser_info(engine_state, stack, "main_decl_id")?;
                Some(DeclId::new(main_decl_id as usize))
            } else {
                None
            };
            (block, main_decl_id)
        };
        let eval_engine_state = full_reparse_engine_state.as_ref().unwrap_or(engine_state);
        let main = main_decl_id.and_then(|decl_id| {
            let decl = eval_engine_state.get_decl(decl_id);
            Some((decl, eval_engine_state.get_block(decl.block_id()?).clone()))
        });

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
            if let Some((main_decl, main_block)) = &main {
                let signature = &main_block.signature;
                let mut callee_stack =
                    stack.gather_captures(eval_engine_state, &main_block.captures);
                // Like a call to a custom command, a call to `main` counts toward the recursion
                // limit, so a script that runs itself fails instead of overflowing the stack.
                callee_stack.recursion_count += 1;
                let mut call_eval = CallEval::new(
                    callee_stack,
                    call.head,
                    main_block.span.unwrap_or(call.head),
                    eval_block_with_early_return,
                );

                // Forward the arguments after the script path (`run file.nu ...args`) to `main`.
                let args = main_arguments(engine_state, stack, call)?;
                if bind_main_arguments(engine_state, signature, args, &mut call_eval)? {
                    let help = get_full_help(*main_decl, eval_engine_state, stack, call.head);
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
) -> Result<(EngineState, Arc<Block>, Option<DeclId>), ShellError> {
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
    let script_main_decl_id = find_main_decl_id_in_script(&working_set, &script_block);
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

    Ok((full_reparse_engine_state, script_block, script_main_decl_id))
}

/// An argument after `run <script>`, evaluated and ready to bind to `main`.
///
/// Each one keeps the span of the argument in the `run` call, so errors point at it and the
/// binder can read its source text.
enum MainArgument {
    Positional(Value, Span),
    Spread(Value, Span),
    Named {
        long: String,
        value: Option<Value>,
        span: Span,
    },
}

impl MainArgument {
    fn span(&self) -> Span {
        match self {
            MainArgument::Positional(_, span)
            | MainArgument::Spread(_, span)
            | MainArgument::Named { span, .. } => *span,
        }
    }
}

/// Evaluate the arguments after the script path of a `run` call.
fn main_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Vec<MainArgument>, ShellError> {
    Ok(match &call.inner {
        CallImpl::IrRef(call) => ir_main_arguments(call.arguments(stack)),
        CallImpl::IrBox(call) => ir_main_arguments(call.arguments(stack)),
        CallImpl::AstRef(call) => ast_main_arguments(engine_state, stack, call)?,
        CallImpl::AstBox(call) => ast_main_arguments(engine_state, stack, call)?,
    })
}

/// The arguments after the script path of a compiled `run` call.
///
/// `run`'s own arguments come first, and the script path is its only positional argument. Flags
/// come from a `--name=value` the parser split off or from a record spread, which the evaluator
/// expanded against `run`'s own signature. Only their long names are kept, so `run`'s short for a
/// flag (`-f` for `full-reparse`) can't match a flag of `main`.
///
/// A flag without a value can only come from a record spread, so it's `true`, like the record
/// field it came from, and never takes the next argument as its value.
fn ir_main_arguments(args: &[engine::Argument]) -> Vec<MainArgument> {
    let flag_name = |data: &[u8], slice| String::from_utf8_lossy(&data[slice]).into_owned();
    args.iter()
        .skip_while(|arg| !matches!(arg, engine::Argument::Positional { .. }))
        .skip(1)
        .filter_map(|arg| match arg {
            engine::Argument::Positional { val, span, .. } => {
                Some(MainArgument::Positional(val.clone(), *span))
            }
            engine::Argument::Spread { vals, span, .. } => {
                Some(MainArgument::Spread(vals.clone(), *span))
            }
            engine::Argument::Flag {
                data, name, span, ..
            } => Some(MainArgument::Named {
                long: flag_name(data, *name),
                value: Some(Value::bool(true, *span)),
                span: *span,
            }),
            engine::Argument::Named {
                data,
                name,
                span,
                val,
                ..
            } => Some(MainArgument::Named {
                long: flag_name(data, *name),
                value: Some(val.clone()),
                span: *span,
            }),
            engine::Argument::ParserInfo { .. } => None,
        })
        .collect()
}

/// Evaluate the arguments after the script path of a `run` call that wasn't compiled.
///
/// `run`'s own arguments come first, and the script path is its only positional argument.
fn ast_main_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &ast::Call,
) -> Result<Vec<MainArgument>, ShellError> {
    let eval_expression = get_eval_expression(engine_state);
    call.arguments
        .iter()
        .skip_while(|arg| !matches!(arg, ast::Argument::Positional(_)))
        .skip(1)
        .map(|arg| {
            Ok(match arg {
                ast::Argument::Positional(expr) | ast::Argument::Unknown(expr) => {
                    MainArgument::Positional(eval_expression(engine_state, stack, expr)?, expr.span)
                }
                ast::Argument::Spread(expr) => {
                    MainArgument::Spread(eval_expression(engine_state, stack, expr)?, expr.span)
                }
                ast::Argument::Named((long, _, expr)) => MainArgument::Named {
                    long: long.item.clone(),
                    value: expr
                        .as_ref()
                        .map(|expr| eval_expression(engine_state, stack, expr))
                        .transpose()?,
                    span: arg.span(),
                },
            })
        })
        .collect()
}

/// A bare argument whose source text is a flag or `--`.
enum FlagToken {
    /// `--`: every argument after it is positional.
    EndOfOptions,
    /// `--name`, with the text after the `=` of a `--name=value` the parser left a word.
    Long(String, Option<String>),
    /// `-c`, or a batch of short flags like `-abc`.
    Short(String),
}

/// Recognize a flag from the source text of an argument in the `run` call, as the parser does.
///
/// Only a bare word can be a flag, so a quoted `"-c"` or a variable holding `"--name"` stays a
/// value. `-abc` is a batch of short flags when each of its characters is a letter or a short flag
/// of `main`, so a negative number like `-1` stays a value.
fn flag_token(engine_state: &EngineState, signature: &Signature, span: Span) -> Option<FlagToken> {
    let token = std::str::from_utf8(engine_state.get_span_contents(span)).ok()?;
    if token == "--" {
        return Some(FlagToken::EndOfOptions);
    }

    if let Some(flag) = token.strip_prefix("--") {
        // The parser splits `--name=value` off, except `--help=...`, `--=...`, and one after a
        // `--` that a flag turns out to take as its value. That value is only text.
        return Some(match flag.split_once('=') {
            Some((name, value)) => FlagToken::Long(name.to_owned(), Some(value.to_owned())),
            None => FlagToken::Long(flag.to_owned(), None),
        });
    }

    let shorts = token.strip_prefix('-')?;
    let is_short_flag =
        |short: char| short.is_ascii_alphabetic() || signature.get_short_flag(short).is_some();
    (!shorts.is_empty() && shorts.chars().all(is_short_flag))
        .then(|| FlagToken::Short(shorts.to_owned()))
}

/// Point a type error for an argument after the script path at that argument, and explain it.
///
/// The parser parses these arguments without `main`'s signature, so a bare `23` is an `int` even
/// where `main` takes a `string`, which a call to `main` would convert. A variable, a
/// subexpression or a record has the same type in a call to `main`, so its error gets no hint.
fn with_conversion_hint(engine_state: &EngineState, err: ShellError, arg_span: Span) -> ShellError {
    match err {
        ShellError::CantConvert {
            to_type,
            from_type,
            help: None,
            ..
        } => {
            let text = engine_state.get_span_contents(arg_span);
            // The value of a `--name=value` is the text after the `=`.
            let value = match text.strip_prefix(b"--") {
                Some(flag) => flag.splitn(2, |&byte| byte == b'=').nth(1).unwrap_or(flag),
                None => text,
            };
            let help = (!matches!(value.first(), Some(b'$' | b'(' | b'{'))).then(|| {
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
                span: arg_span,
                help,
            }
        }
        err => err,
    }
}

/// Bind the arguments after the script path to the script's `main`.
///
/// The parser parsed each argument as `any`, before the script and without `main`'s signature,
/// splitting `--name=value` into a named argument with a parsed value. Here a bare word is a flag
/// when its source text is one, as for the parser, so a quoted `"-c"`, a variable holding
/// `"--name"` and the items of a spread list stay values. What a call to `main` rejects is an
/// error here too: unknown flags, flags missing their value, values of the wrong type, and extra
/// positional arguments. Unlike a call to `main`, values aren't converted to the parameter's type,
/// so `--name 42` for a `string` flag is an error that says to quote it.
///
/// Returns `true` when the arguments ask for `main`'s help (`--help` or `-h`).
fn bind_main_arguments(
    engine_state: &EngineState,
    signature: &Signature,
    args: Vec<MainArgument>,
    call_eval: &mut CallEval,
) -> Result<bool, ShellError> {
    let positional_len = signature.required_positional.len() + signature.optional_positional.len();
    let mut positional_count = 0;
    let mut add_positional =
        |call_eval: &mut CallEval, value: Value, span: Span| -> Result<(), ShellError> {
            if positional_count >= positional_len && signature.rest_positional.is_none() {
                return Err(ShellError::from_diagnostic(&ParseError::ExtraPositional(
                    signature.call_signature(),
                    span,
                )));
            }
            positional_count += 1;
            call_eval
                .add_positional(signature, Cow::Owned(value))
                .map_err(|err| with_conversion_hint(engine_state, err, span))?;
            Ok(())
        };
    let mut end_of_options = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let span = arg.span();
        // The flag the argument names, how to name it in an error, and the value written with it
        // (`--name=value`).
        let (flag, name, inline_value) = match arg {
            MainArgument::Spread(value, _) => {
                // `null` adds nothing to a `main` without a rest parameter, so the arguments after
                // it still fill its positionals.
                if !value.is_nothing() || can_rest_spread(signature) {
                    call_eval.add_spread(signature, value, span)?;
                }
                continue;
            }
            // `--name=value`, which the parser splits off only before a `--`, or a flag from a
            // record spread.
            MainArgument::Named { long, value, .. } => {
                (signature.get_long_flag(&long), long, value)
            }
            MainArgument::Positional(value, _) => {
                let token = if end_of_options {
                    None
                } else {
                    flag_token(engine_state, signature, span)
                };
                match token {
                    None => {
                        add_positional(&mut *call_eval, value, span)?;
                        continue;
                    }
                    Some(FlagToken::EndOfOptions) => {
                        end_of_options = true;
                        // A wrapped `main` gets the `--` too, like a call to it does.
                        if signature.allows_unknown_args {
                            add_positional(&mut *call_eval, value, span)?;
                        }
                        continue;
                    }
                    Some(FlagToken::Long(name, inline)) => (
                        signature.get_long_flag(&name),
                        name,
                        inline.map(|text| Value::string(text, span)),
                    ),
                    Some(FlagToken::Short(shorts)) => {
                        // Like a call to `main`, `-abc` sets each flag it names and only the last
                        // one can take a value. An unknown one makes the whole word unknown.
                        match shorts
                            .chars()
                            .map(|short| signature.get_short_flag(short).ok_or(short))
                            .collect::<Result<Vec<_>, _>>()
                        {
                            Err(short) => (None, format!("-{short}"), None),
                            Ok(mut flags) => {
                                let last = flags.pop();
                                for flag in flags {
                                    if flag.long == "help" {
                                        return Ok(true);
                                    }
                                    if flag.arg.is_some() {
                                        return Err(ShellError::from_diagnostic(
                                            &ParseError::OnlyLastFlagInBatchCanTakeArg(span),
                                        ));
                                    }
                                    call_eval.add_named(
                                        signature,
                                        &flag.long,
                                        flag.short.map(String::from),
                                        None,
                                    )?;
                                }
                                (last, format!("-{shorts}"), None)
                            }
                        }
                    }
                }
            }
        };

        let Some(flag) = flag else {
            // A wrapped `main` takes an unknown flag as an argument, like a call to it does. A flag
            // from a record spread isn't a word written in the call, so it stays unknown.
            let text = String::from_utf8_lossy(engine_state.get_span_contents(span));
            if signature.allows_unknown_args && text.starts_with('-') {
                add_positional(&mut *call_eval, Value::string(text, span), span)?;
                continue;
            }
            // Like the parser, only suggest a name for a long flag.
            let help = if name.starts_with('-') {
                None
            } else {
                did_you_mean(signature.get_names(), &name)
                    .map(|name| format!("Did you mean: `--{name}`?"))
            };
            return Err(ShellError::from_diagnostic(&ParseError::UnknownFlag(
                signature.name.clone(),
                name,
                span,
                help.unwrap_or_else(|| "Use `--help` to see available flags".to_owned()),
            )));
        };

        // Like a call to `main`, `--help` shows its help, even when `main` declares the flag.
        if flag.long == "help" {
            return Ok(true);
        }

        let (value, value_span) = match (&flag.arg, inline_value) {
            // A switch is `--switch`, or `--switch=true` and `--switch=false`.
            (None, value) | (Some(_), value @ Some(_)) => (value, span),
            // The flag's value is the next argument, as in a call to `main`.
            (Some(shape), None) => match args.next() {
                Some(MainArgument::Positional(value, value_span)) => (Some(value), value_span),
                Some(arg) => {
                    // The value is the argument as written. The span of a spread, or of a flag
                    // from a record spread, starts after its `...`.
                    let value_span = arg.span();
                    let text = String::from_utf8_lossy(engine_state.get_span_contents(value_span));
                    let text = if text.starts_with('-') {
                        text.into_owned()
                    } else {
                        format!("...{text}")
                    };
                    (Some(Value::string(text, value_span)), value_span)
                }
                None => {
                    return Err(ShellError::from_diagnostic(&ParseError::MissingFlagParam(
                        shape.to_string(),
                        span,
                    )));
                }
            },
        };

        if let Some(value) = &value {
            // Like a call to `main`, `null` leaves out a flag whose type doesn't take it.
            if value.is_nothing() && !flag.type_accepts_nothing() {
                continue;
            }
            let flag_type = flag.arg.as_ref().map_or(Type::Bool, SyntaxShape::to_type);
            if !value.is_subtype_of(&flag_type) {
                let err = ShellError::CantConvert {
                    to_type: flag_type.to_string(),
                    from_type: value.get_type().to_string(),
                    span: value_span,
                    help: None,
                };
                return Err(with_conversion_hint(engine_state, err, value_span));
            }
        }

        call_eval.add_named(
            signature,
            &flag.long,
            flag.short.map(String::from),
            value.map(Cow::Owned),
        )?;
    }

    Ok(false)
}
