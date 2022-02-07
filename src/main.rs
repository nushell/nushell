<<<<<<< HEAD
use nu_cli::App as CliApp;
use nu_errors::ShellError;

fn main() -> Result<(), ShellError> {
    let mut argv = vec![String::from("nu")];
    argv.extend(positionals());

    CliApp::run(&argv)
}

fn positionals() -> Vec<String> {
    std::env::args().skip(1).collect::<Vec<_>>()
=======
mod commands;
mod config_files;
mod eval_file;
mod logger;
mod prompt_update;
mod reedline_config;
mod repl;
mod utils;

#[cfg(test)]
mod tests;

mod test_bins;

use miette::Result;
use nu_command::{create_default_context, BufferedReader};
use nu_engine::get_full_help;
use nu_parser::parse;
use nu_protocol::{
    ast::{Call, Expr, Expression, Pipeline, Statement},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value, CONFIG_VARIABLE_ID,
};
use std::{
    io::{BufReader, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use utils::report_error;

fn main() -> Result<()> {
    // miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    // Get initial current working directory.
    let init_cwd = utils::get_init_cwd();
    let mut engine_state = create_default_context(&init_cwd);

    // Custom additions
    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(nu_cli::NuHighlight));

        working_set.render()
    };
    let _ = engine_state.merge_delta(delta, None, &init_cwd);

    // TODO: make this conditional in the future
    // Ctrl-c protection section
    let ctrlc = Arc::new(AtomicBool::new(false));
    let handler_ctrlc = ctrlc.clone();
    let engine_state_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(engine_state_ctrlc);
    // End ctrl-c protection section

    let mut args_to_nushell = vec![];
    let mut script_name = String::new();
    let mut args_to_script = vec![];

    // Would be nice if we had a way to parse this. The first flags we see will be going to nushell
    // then it'll be the script name
    // then the args to the script

    let mut collect_arg_nushell = false;
    for arg in std::env::args().skip(1) {
        if !script_name.is_empty() {
            args_to_script.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
        } else if collect_arg_nushell {
            args_to_nushell.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
            collect_arg_nushell = false;
        } else if arg.starts_with('-') {
            // Cool, it's a flag
            if arg == "-c"
                || arg == "--commands"
                || arg == "--testbin"
                || arg == "--develop"
                || arg == "--debug"
                || arg == "--loglevel"
                || arg == "--config-file"
            {
                collect_arg_nushell = true;
            }
            args_to_nushell.push(arg);
        } else {
            // Our script file
            script_name = arg;
        }
    }

    args_to_nushell.insert(0, "nu".into());

    let nushell_commandline_args = args_to_nushell.join(" ");

    let nushell_config =
        parse_commandline_args(&nushell_commandline_args, &init_cwd, &mut engine_state);

    match nushell_config {
        Ok(nushell_config) => {
            if let Some(testbin) = &nushell_config.testbin {
                // Call out to the correct testbin
                match testbin.item.as_str() {
                    "echo_env" => test_bins::echo_env(),
                    "cococo" => test_bins::cococo(),
                    "meow" => test_bins::meow(),
                    "iecho" => test_bins::iecho(),
                    "fail" => test_bins::fail(),
                    "nonu" => test_bins::nonu(),
                    "chop" => test_bins::chop(),
                    "repeater" => test_bins::repeater(),
                    _ => std::process::exit(1),
                }
                std::process::exit(0)
            }
            let input = if let Some(redirect_stdin) = &nushell_config.redirect_stdin {
                let stdin = std::io::stdin();
                let buf_reader = BufReader::new(stdin);

                PipelineData::RawStream(
                    RawStream::new(
                        Box::new(BufferedReader::new(buf_reader)),
                        Some(ctrlc),
                        redirect_stdin.span,
                    ),
                    redirect_stdin.span,
                    None,
                )
            } else {
                PipelineData::new(Span::new(0, 0))
            };

            if let Some(commands) = &nushell_config.commands {
                commands::evaluate(commands, &init_cwd, &mut engine_state, input)
            } else if !script_name.is_empty() && nushell_config.interactive_shell.is_none() {
                eval_file::evaluate(
                    script_name,
                    &args_to_script,
                    init_cwd,
                    &mut engine_state,
                    input,
                )
            } else {
                repl::evaluate(&mut engine_state)
            }
        }
        Err(_) => std::process::exit(1),
    }
}

fn parse_commandline_args(
    commandline_args: &str,
    init_cwd: &Path,
    engine_state: &mut EngineState,
) -> Result<NushellConfig, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.add_decl(Box::new(Nu));

        let (output, err) = parse(&mut working_set, None, commandline_args.as_bytes(), false);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"nu");
        (output, working_set.render())
    };

    let _ = engine_state.merge_delta(delta, None, init_cwd);

    let mut stack = Stack::new();
    stack.add_var(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    // We should have a successful parse now
    if let Some(Statement::Pipeline(Pipeline { expressions })) = block.stmts.get(0) {
        if let Some(Expression {
            expr: Expr::Call(call),
            ..
        }) = expressions.get(0)
        {
            let redirect_stdin = call.get_named_arg("stdin");
            let login_shell = call.get_named_arg("login");
            let interactive_shell = call.get_named_arg("interactive");
            let commands: Option<Expression> = call.get_flag_expr("commands");
            let testbin: Option<Expression> = call.get_flag_expr("testbin");

            let commands = if let Some(expression) = commands {
                let contents = engine_state.get_span_contents(&expression.span);

                Some(Spanned {
                    item: String::from_utf8_lossy(contents).to_string(),
                    span: expression.span,
                })
            } else {
                None
            };

            let testbin = if let Some(expression) = testbin {
                let contents = engine_state.get_span_contents(&expression.span);

                Some(Spanned {
                    item: String::from_utf8_lossy(contents).to_string(),
                    span: expression.span,
                })
            } else {
                None
            };

            let help = call.has_flag("help");

            if help {
                let full_help =
                    get_full_help(&Nu.signature(), &Nu.examples(), engine_state, &mut stack);

                let _ = std::panic::catch_unwind(move || {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let _ = stdout.write_all(full_help.as_bytes());
                });

                std::process::exit(1);
            }

            return Ok(NushellConfig {
                redirect_stdin,
                login_shell,
                interactive_shell,
                commands,
                testbin,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(&Nu.signature(), &Nu.examples(), engine_state, &mut stack);
    print!("{}", full_help);
    std::process::exit(1);
}

struct NushellConfig {
    redirect_stdin: Option<Spanned<String>>,
    #[allow(dead_code)]
    login_shell: Option<Spanned<String>>,
    interactive_shell: Option<Spanned<String>>,
    commands: Option<Spanned<String>>,
    testbin: Option<Spanned<String>>,
}

#[derive(Clone)]
struct Nu;

impl Command for Nu {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu")
            .desc("The nushell language and shell.")
            .switch("stdin", "redirect the stdin", None)
            .switch("login", "start as a login shell", Some('l'))
            .switch("interactive", "start as an interactive shell", Some('i'))
            .named(
                "testbin",
                SyntaxShape::String,
                "run internal test binary",
                None,
            )
            .named(
                "commands",
                SyntaxShape::String,
                "run the given commands and then exit",
                Some('c'),
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
            .category(Category::System)
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Nu.signature(), &Nu.examples(), engine_state, stack),
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
