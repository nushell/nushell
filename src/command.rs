use nu_engine::{get_full_help, CallExt};
use nu_parser::parse;
use nu_parser::{escape_for_script_arg, escape_quote_string};
use nu_protocol::report_error;
use nu_protocol::{
    ast::{Call, Expr, Expression, PipelineElement},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Value,
};
use nu_utils::stdout_write_all_and_flush;

pub(crate) fn gather_commandline_args() -> (Vec<String>, String, Vec<String>) {
    // Would be nice if we had a way to parse this. The first flags we see will be going to nushell
    // then it'll be the script name
    // then the args to the script
    let mut args_to_nushell = Vec::from(["nu".into()]);
    let mut script_name = String::new();
    let mut args = std::env::args();

    // Mimic the behaviour of bash/zsh
    if let Some(argv0) = args.next() {
        if argv0.starts_with('-') {
            args_to_nushell.push("--login".into());
        }
    }

    while let Some(arg) = args.next() {
        if !arg.starts_with('-') {
            script_name = arg;
            break;
        }

        let flag_value = match arg.as_ref() {
            "--commands" | "-c" | "--table-mode" | "-m" | "-e" | "--execute" | "--config"
            | "--env-config" | "-I" | "ide-ast" => args.next().map(|a| escape_quote_string(&a)),
            #[cfg(feature = "plugin")]
            "--plugin-config" => args.next().map(|a| escape_quote_string(&a)),
            "--log-level" | "--log-target" | "--testbin" | "--threads" | "-t"
            | "--include-path" | "--ide-goto-def" | "--ide-hover" | "--ide-complete"
            | "--ide-check" => args.next(),
            _ => None,
        };

        args_to_nushell.push(arg);

        if let Some(flag_value) = flag_value {
            args_to_nushell.push(flag_value);
        }
    }

    let args_to_script = if !script_name.is_empty() {
        args.map(|arg| escape_for_script_arg(&arg)).collect()
    } else {
        Vec::default()
    };
    (args_to_nushell, script_name, args_to_script)
}

pub(crate) fn parse_commandline_args(
    commandline_args: &str,
    engine_state: &mut EngineState,
) -> Result<NushellCliArgs, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.add_decl(Box::new(Nu));

        let output = parse(&mut working_set, None, commandline_args.as_bytes(), false);
        if let Some(err) = working_set.parse_errors.first() {
            report_error(&working_set, err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"nu");
        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let mut stack = Stack::new();

    // We should have a successful parse now
    if let Some(pipeline) = block.pipelines.get(0) {
        if let Some(PipelineElement::Expression(
            _,
            Expression {
                expr: Expr::Call(call),
                ..
            },
        )) = pipeline.elements.get(0)
        {
            let redirect_stdin = call.get_named_arg("stdin");
            let login_shell = call.get_named_arg("login");
            let interactive_shell = call.get_named_arg("interactive");
            let commands: Option<Expression> = call.get_flag_expr("commands");
            let testbin: Option<Expression> = call.get_flag_expr("testbin");
            #[cfg(feature = "plugin")]
            let plugin_file: Option<Expression> = call.get_flag_expr("plugin-config");
            let no_config_file = call.get_named_arg("no-config-file");
            let no_std_lib = call.get_named_arg("no-std-lib");
            let config_file: Option<Expression> = call.get_flag_expr("config");
            let env_file: Option<Expression> = call.get_flag_expr("env-config");
            let log_level: Option<Expression> = call.get_flag_expr("log-level");
            let log_target: Option<Expression> = call.get_flag_expr("log-target");
            let execute: Option<Expression> = call.get_flag_expr("execute");
            let table_mode: Option<Value> =
                call.get_flag(engine_state, &mut stack, "table-mode")?;

            // ide flags
            let include_path: Option<Expression> = call.get_flag_expr("include-path");
            let ide_goto_def: Option<Value> =
                call.get_flag(engine_state, &mut stack, "ide-goto-def")?;
            let ide_hover: Option<Value> = call.get_flag(engine_state, &mut stack, "ide-hover")?;
            let ide_complete: Option<Value> =
                call.get_flag(engine_state, &mut stack, "ide-complete")?;
            let ide_check: Option<Value> = call.get_flag(engine_state, &mut stack, "ide-check")?;
            let ide_ast: Option<Spanned<String>> = call.get_named_arg("ide-ast");

            fn extract_contents(
                expression: Option<Expression>,
            ) -> Result<Option<Spanned<String>>, ShellError> {
                if let Some(expr) = expression {
                    let str = expr.as_string();
                    if let Some(str) = str {
                        Ok(Some(Spanned {
                            item: str,
                            span: expr.span,
                        }))
                    } else {
                        Err(ShellError::TypeMismatch {
                            err_message: "string".into(),
                            span: expr.span,
                        })
                    }
                } else {
                    Ok(None)
                }
            }

            let commands = extract_contents(commands)?;
            let testbin = extract_contents(testbin)?;
            #[cfg(feature = "plugin")]
            let plugin_file = extract_contents(plugin_file)?;
            let config_file = extract_contents(config_file)?;
            let env_file = extract_contents(env_file)?;
            let log_level = extract_contents(log_level)?;
            let log_target = extract_contents(log_target)?;
            let execute = extract_contents(execute)?;
            let include_path = extract_contents(include_path)?;

            let help = call.has_flag("help");

            if help {
                let full_help = get_full_help(
                    &Nu.signature(),
                    &Nu.examples(),
                    engine_state,
                    &mut stack,
                    true,
                );

                let _ = std::panic::catch_unwind(move || stdout_write_all_and_flush(full_help));

                std::process::exit(0);
            }

            if call.has_flag("version") {
                let version = env!("CARGO_PKG_VERSION").to_string();
                let _ = std::panic::catch_unwind(move || {
                    stdout_write_all_and_flush(format!("{version}\n"))
                });

                std::process::exit(0);
            }

            return Ok(NushellCliArgs {
                redirect_stdin,
                login_shell,
                interactive_shell,
                commands,
                testbin,
                #[cfg(feature = "plugin")]
                plugin_file,
                no_config_file,
                no_std_lib,
                config_file,
                env_file,
                log_level,
                log_target,
                execute,
                include_path,
                ide_goto_def,
                ide_hover,
                ide_complete,
                ide_check,
                ide_ast,
                table_mode,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(
        &Nu.signature(),
        &Nu.examples(),
        engine_state,
        &mut stack,
        true,
    );
    print!("{full_help}");
    std::process::exit(1);
}

pub(crate) struct NushellCliArgs {
    pub(crate) redirect_stdin: Option<Spanned<String>>,
    pub(crate) login_shell: Option<Spanned<String>>,
    pub(crate) interactive_shell: Option<Spanned<String>>,
    pub(crate) commands: Option<Spanned<String>>,
    pub(crate) testbin: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    pub(crate) plugin_file: Option<Spanned<String>>,
    pub(crate) no_config_file: Option<Spanned<String>>,
    pub(crate) no_std_lib: Option<Spanned<String>>,
    pub(crate) config_file: Option<Spanned<String>>,
    pub(crate) env_file: Option<Spanned<String>>,
    pub(crate) log_level: Option<Spanned<String>>,
    pub(crate) log_target: Option<Spanned<String>>,
    pub(crate) execute: Option<Spanned<String>>,
    pub(crate) table_mode: Option<Value>,
    pub(crate) include_path: Option<Spanned<String>>,
    pub(crate) ide_goto_def: Option<Value>,
    pub(crate) ide_hover: Option<Value>,
    pub(crate) ide_complete: Option<Value>,
    pub(crate) ide_check: Option<Value>,
    pub(crate) ide_ast: Option<Spanned<String>>,
}

#[derive(Clone)]
struct Nu;

impl Command for Nu {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        let mut signature = Signature::build("nu")
            .usage("The nushell language and shell.")
            .named(
                "commands",
                SyntaxShape::String,
                "run the given commands and then exit",
                Some('c'),
            )
            .named(
                "execute",
                SyntaxShape::String,
                "run the given commands and then enter an interactive shell",
                Some('e'),
            )
            .named(
                "include-path",
                SyntaxShape::String,
                "set the NU_LIB_DIRS for the given script (semicolon-delimited)",
                Some('I'),
            )
            .switch("interactive", "start as an interactive shell", Some('i'))
            .switch("login", "start as a login shell", Some('l'))
            .named(
                "table-mode",
                SyntaxShape::String,
                "the table mode to use. rounded is default.",
                Some('m'),
            )
            .switch(
                "no-config-file",
                "start with no config file and no env file",
                Some('n'),
            )
            .switch("no-std-lib", "start with no standard library", None)
            .named(
                "threads",
                SyntaxShape::Int,
                "threads to use for parallel commands",
                Some('t'),
            )
            .switch("version", "print the version", Some('v'))
            .named(
                "config",
                SyntaxShape::String,
                "start with an alternate config file",
                None,
            )
            .named(
                "env-config",
                SyntaxShape::String,
                "start with an alternate environment config file",
                None,
            )
            .named(
                "ide-goto-def",
                SyntaxShape::Int,
                "go to the definition of the item at the given position",
                None,
            )
            .named(
                "ide-hover",
                SyntaxShape::Int,
                "give information about the item at the given position",
                None,
            )
            .named(
                "ide-complete",
                SyntaxShape::Int,
                "list completions for the item at the given position",
                None,
            )
            .named(
                "ide-check",
                SyntaxShape::Int,
                "run a diagnostic check on the given source",
                None,
            )
            .switch("ide-ast", "generate the ast on the given source", None);

        #[cfg(feature = "plugin")]
        {
            signature = signature.named(
                "plugin-config",
                SyntaxShape::String,
                "start with an alternate plugin signature file",
                None,
            );
        }

        signature = signature
            .named(
                "log-level",
                SyntaxShape::String,
                "log level for diagnostic logs (error, warn, info, debug, trace). Off by default",
                None,
            )
            .named(
                "log-target",
                SyntaxShape::String,
                "set the target for the log to output. stdout, stderr(default), mixed or file",
                None,
            )
            .switch(
                "stdin",
                "redirect standard input to a command (with `-c`) or a script file",
                None,
            )
            .named(
                "testbin",
                SyntaxShape::String,
                "run internal test binary",
                None,
            )
            .optional(
                "script file",
                SyntaxShape::Filepath,
                "name of the optional script file to run",
            )
            .rest(
                "script args",
                SyntaxShape::String,
                "parameters to the script file",
            )
            .category(Category::System);

        signature
    }

    fn usage(&self) -> &str {
        "The nushell language and shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&Nu.signature(), &Nu.examples(), engine_state, stack, true),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Run a script",
                example: "nu myfile.nu",
                result: None,
            },
            Example {
                description: "Run nushell interactively (as a shell or REPL)",
                example: "nu",
                result: None,
            },
        ]
    }
}
