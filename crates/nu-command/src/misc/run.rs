use nu_engine::{
    CallEval, command_prelude::*, get_eval_block_with_early_return, get_eval_expression,
    get_full_help,
};
use nu_parser::{find_main_block_id_in_script, parse};
use nu_path::{absolute_with, is_windows_device_path};
use nu_protocol::{
    BlockId, CompareTypes, DeclId, ParseError, Value,
    ast::{self, Block},
    did_you_mean,
    engine::{
        self, CallImpl, CommandType, StateWorkingSet,
        named_flags::{
            can_rest_spread, data_from_name_and_short, find_signature_flag,
            list_spread_before_required_error, normalize_engine_arguments,
        },
    },
    parser_path::{MAX_RUN_SCRIPT_BYTES, ScriptLoadError, read_run_script_file},
    shell_error::{generic::GenericError, io::IoError},
};
use std::{borrow::Cow, cell::Cell, sync::Arc};

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
                if bind_main_arguments(engine_state, signature, args.main, &mut call_eval)? {
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

/// An argument after `run <script>`, evaluated and ready to bind to `main`.
///
/// Each one keeps the span of the argument in the `run` call, so errors point at it and the
/// binder can read its source text.
enum MainArgument {
    Positional(Value, Span),
    Spread(Value, Span),
    /// A `--name=value` that the parser split off.
    Named {
        long: String,
        value: Value,
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

/// The evaluated arguments of a `run` call.
struct RunArguments {
    /// Whether a record spread before the script path turns on `--full-reparse`, as in
    /// `run ...$opts x.nu`. The parser reads a `--full-reparse` written as a flag.
    full_reparse: bool,
    /// The arguments after the script path, for `main`.
    main: Vec<MainArgument>,
}

/// Evaluate the arguments of a `run` call.
fn run_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<RunArguments, ShellError> {
    match &call.inner {
        CallImpl::IrRef(call) => ir_run_arguments(call.arguments(stack)),
        CallImpl::IrBox(call) => ir_run_arguments(call.arguments(stack)),
        CallImpl::AstRef(call) => ast_run_arguments(engine_state, stack, call),
        CallImpl::AstBox(call) => ast_run_arguments(engine_state, stack, call),
    }
}

/// Whether `run`'s own arguments, the ones before the script path, turn on `--full-reparse`.
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

/// The value of a spread or a `--name=value` after the script path, which the parser wraps in a
/// one-item list.
fn unwrap_script_argument(wrapped: Value, span: Span) -> Result<Value, ShellError> {
    let item = match wrapped {
        Value::List { vals, .. } => <[Value; 1]>::try_from(vals.into_owned()).ok(),
        _ => None,
    };
    item.map(|[value]| value)
        .ok_or_else(|| ShellError::NushellFailedSpanned {
            msg: "`run` expected the parser to wrap a script argument in a list".into(),
            label: "this argument".into(),
            span,
        })
}

/// The arguments of a `run` call, as the IR evaluator passes them.
///
/// `run`'s own arguments come first, and the script path is its only positional argument. After
/// the path, a named argument is a `--name=value` the parser split off.
fn ir_run_arguments(args: &[engine::Argument]) -> Result<RunArguments, ShellError> {
    let path = args
        .iter()
        .position(|arg| matches!(arg, engine::Argument::Positional { .. }))
        .unwrap_or(args.len());
    let main =
        args.iter()
            .skip(path + 1)
            .filter_map(|arg| match arg {
                engine::Argument::Positional { val, span, .. } => {
                    Some(Ok(MainArgument::Positional(val.clone(), *span)))
                }
                engine::Argument::Spread { vals, span, .. } => Some(
                    unwrap_script_argument(vals.clone(), *span)
                        .map(|value| MainArgument::Spread(value, *span)),
                ),
                engine::Argument::Named {
                    data,
                    name,
                    span,
                    val,
                    ..
                } => Some(unwrap_script_argument(val.clone(), *span).map(|value| {
                    MainArgument::Named {
                        long: String::from_utf8_lossy(&data[*name]).into_owned(),
                        value,
                        span: *span,
                    }
                })),
                // The parser adds no flag without a value after the path.
                engine::Argument::Flag { .. } | engine::Argument::ParserInfo { .. } => None,
            })
            .collect::<Result<_, _>>()?;

    Ok(RunArguments {
        full_reparse: spread_full_reparse(&args[..path])?,
        main,
    })
}

/// Evaluate the arguments of a `run` call that wasn't compiled, as the IR evaluator would pass
/// them: with record spreads expanded and `null` values left out against `run`'s signature.
fn ast_run_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &ast::Call,
) -> Result<RunArguments, ShellError> {
    let eval_expression = get_eval_expression(engine_state);
    let args = call
        .arguments
        .iter()
        .map(|arg| {
            let span = arg.span();
            Ok(match arg {
                ast::Argument::Positional(expr) | ast::Argument::Unknown(expr) => {
                    engine::Argument::Positional {
                        span,
                        val: eval_expression(engine_state, stack, expr)?,
                        ast: None,
                    }
                }
                ast::Argument::Spread(expr) => engine::Argument::Spread {
                    span,
                    vals: eval_expression(engine_state, stack, expr)?,
                    ast: None,
                },
                ast::Argument::Named((long, short, expr)) => {
                    let short = short.as_ref().map_or("", |short| short.item.as_str());
                    let (data, name, short) = data_from_name_and_short(&long.item, short);
                    match expr {
                        Some(expr) => engine::Argument::Named {
                            data,
                            name,
                            short,
                            span,
                            val: eval_expression(engine_state, stack, expr)?,
                            ast: None,
                        },
                        None => engine::Argument::Flag {
                            data,
                            name,
                            short,
                            span,
                        },
                    }
                }
            })
        })
        .collect::<Result<_, ShellError>>()?;
    ir_run_arguments(&normalize_engine_arguments(&Run.signature(), args)?)
}

/// A bare argument whose source text is a flag or `--`.
enum FlagToken<'a> {
    /// `--`: every argument after it is positional.
    EndOfOptions,
    /// `--name`, with the text after the `=` of a `--name=value` the parser left a word.
    Long(&'a str, Option<&'a str>),
    /// A batch of short flags like `-abc`: the flags before the last one, and the last one, which
    /// can take a value.
    Short(Vec<&'a Flag>, &'a Flag),
    /// A batch of short flags with a character that isn't a short flag of `main`.
    UnknownShort(char),
}

/// Recognize a flag from the source text of an argument in the `run` call, as the parser does.
///
/// Only a bare word can be a flag, so a quoted `"-c"` or a variable holding `"--name"` stays a
/// value. `-abc` is a batch of short flags, and a character in it that isn't a short flag of `main`
/// makes it an unknown flag, so a mistyped `-dry-run` is an error rather than a value. A word that
/// names no short flag of `main` stays a value when the parser read it as a number, filesize,
/// duration or range, like `-1`, `-inf`, `-5kb` or `-1..5`. The parser read it without `main`'s
/// signature, so this is looser than a call to `main`, which takes only numbers, and only where
/// the parameter takes a number.
fn flag_token<'a>(
    engine_state: &'a EngineState,
    signature: &'a Signature,
    value: &Value,
    span: Span,
) -> Option<FlagToken<'a>> {
    let token = std::str::from_utf8(engine_state.get_span_contents(span)).ok()?;
    if token == "--" {
        return Some(FlagToken::EndOfOptions);
    }

    if let Some(flag) = token.strip_prefix("--") {
        // The parser splits `--name=value` off, except `--help=...`, `--=...`, and one after a
        // `--` that a flag turns out to take as its value. That value is only text.
        return Some(match flag.split_once('=') {
            Some((name, value)) => FlagToken::Long(name, Some(value)),
            None => FlagToken::Long(flag, None),
        });
    }

    let shorts = token
        .strip_prefix('-')
        .filter(|shorts| !shorts.is_empty())?;
    let mut flags = Vec::new();
    let mut unknown = None;
    for short in shorts.chars() {
        match signature
            .named
            .iter()
            .find(|flag| flag.short == Some(short))
        {
            Some(flag) => flags.push(flag),
            None => {
                unknown.get_or_insert(short);
            }
        }
    }
    if flags.is_empty()
        && matches!(
            value,
            Value::Int { .. }
                | Value::Float { .. }
                | Value::Filesize { .. }
                | Value::Duration { .. }
                | Value::Range { .. }
        )
    {
        return None;
    }
    Some(match unknown {
        Some(short) => FlagToken::UnknownShort(short),
        None => {
            let last = flags.pop()?;
            FlagToken::Short(flags, last)
        }
    })
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

/// Bind the arguments after the script path to the script's `main`.
///
/// The parser parsed each argument as `any`, before the script and without `main`'s signature,
/// splitting `--name=value` into a named argument with a parsed value. Here a bare word is a flag
/// when its source text is one, as for the parser, so a quoted `"-c"`, a variable holding
/// `"--name"` and the items of a spread list stay values, while a spread record sets flags. What a
/// call to `main` rejects is an error here too: unknown flags, flags missing their value, values of
/// the wrong type, and extra positional arguments. Unlike a call to `main`, values aren't converted
/// to the parameter's type, so `--name 42` for a `string` flag is an error that says to quote it.
///
/// Returns `true` when the arguments ask for `main`'s help (`--help`, `-h` or `...{help: true}`).
/// As in a call to `main`, the other arguments are checked first.
fn bind_main_arguments(
    engine_state: &EngineState,
    signature: &Signature,
    args: Vec<MainArgument>,
    call_eval: &mut CallEval,
) -> Result<bool, ShellError> {
    let find_long_flag = |name: &str| find_signature_flag(signature, name.as_bytes(), b"");
    let positional_count = Cell::new(0);
    let add_positional =
        |call_eval: &mut CallEval, value: Value, span: Span| -> Result<(), ShellError> {
            if signature.get_positional(positional_count.get()).is_none() {
                return Err(ShellError::from_diagnostic(&ParseError::ExtraPositional(
                    signature.call_signature(),
                    span,
                )));
            }
            positional_count.set(positional_count.get() + 1);
            call_eval
                .add_positional(signature, Cow::Owned(value))
                .map_err(|err| with_conversion_hint(engine_state, err, span))?;
            Ok(())
        };
    let mut help = false;
    let mut end_of_options = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let span = arg.span();
        // The flag the argument names, or its name for an error, and the value written with it
        // (`--name=value`) with the span of that value.
        let (flag, inline_value) = match arg {
            MainArgument::Spread(mut value, _) => {
                // `...{help: true}` asks for `main`'s help, like `--help`, when `main` has one.
                if let Value::Record { val, .. } = &mut value
                    && matches!(val.get("help"), Some(Value::Bool { val: true, .. }))
                    && find_long_flag("help").is_some()
                {
                    help = true;
                    val.to_mut().remove("help");
                }
                // A record sets `main`'s flags. `null` adds nothing to a `main` without a rest
                // parameter, so the arguments after it still fill its positionals.
                if !value.is_nothing() || can_rest_spread(signature) {
                    call_eval
                        .add_spread(signature, value, span)
                        .map_err(|err| with_conversion_hint(engine_state, err, span))?;
                }
                continue;
            }
            // `--name=value`, which the parser splits off only before a `--`.
            MainArgument::Named { long, value, .. } => {
                let value_span = Span::new(span.start + long.len() + 3, span.end);
                (find_long_flag(&long).ok_or(long), Some((value, value_span)))
            }
            MainArgument::Positional(value, _) => {
                let token = if end_of_options {
                    None
                } else {
                    flag_token(engine_state, signature, &value, span)
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
                        find_long_flag(name).ok_or_else(|| name.to_owned()),
                        inline.map(|text| {
                            let value_span = Span::new(span.start + name.len() + 3, span.end);
                            (Value::string(text, value_span), value_span)
                        }),
                    ),
                    Some(FlagToken::UnknownShort(short)) => (Err(format!("-{short}")), None),
                    Some(FlagToken::Short(leading, last)) => {
                        // Like a call to `main`, `-abc` sets each flag it names, and only the last
                        // one can take a value.
                        for flag in leading {
                            if flag.arg.is_some() {
                                return Err(ShellError::from_diagnostic(
                                    &ParseError::OnlyLastFlagInBatchCanTakeArg(span),
                                ));
                            }
                            if flag.long == "help" {
                                help = true;
                            } else {
                                call_eval.add_named(
                                    signature,
                                    &flag.long,
                                    flag.short.map(String::from),
                                    None,
                                )?;
                            }
                        }
                        (Ok(last), None)
                    }
                }
            }
        };

        let flag = match flag {
            Ok(flag) => flag,
            // A wrapped `main` takes an unknown flag as an argument, as it's written.
            Err(_) if signature.allows_unknown_args => {
                let text = String::from_utf8_lossy(engine_state.get_span_contents(span));
                add_positional(&mut *call_eval, Value::string(text, span), span)?;
                continue;
            }
            Err(name) => {
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
            }
        };

        // Like a call to `main`, `--help` asks for its help, even when `main` declares the flag.
        if flag.long == "help" {
            help = true;
            continue;
        }

        let (value, value_span) = match (&flag.arg, inline_value) {
            // `--name=value`, or `--switch=true` and `--switch=false`.
            (_, Some((value, value_span))) => (Some(value), value_span),
            (None, None) => (None, span),
            // The flag's value is the next argument, as in a call to `main`.
            (Some(shape), None) => match args.next() {
                Some(MainArgument::Positional(value, value_span)) => (Some(value), value_span),
                // A `--name=value` or a spread is the value as it's written. The span of a spread
                // starts after its `...`.
                Some(arg) => {
                    let value_span = arg.span();
                    let text = String::from_utf8_lossy(engine_state.get_span_contents(value_span));
                    let text = if matches!(arg, MainArgument::Spread(..)) {
                        format!("...{text}")
                    } else {
                        text.into_owned()
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
            // `null` leaves out a flag whose type doesn't take it, as a `null` variable does in a
            // call to `main`. (The parser rejects a written `null` there.)
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

    Ok(help)
}
