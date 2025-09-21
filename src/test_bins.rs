use nu_cmd_base::hook::{eval_env_change_hook, eval_hooks};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    PipelineData, ShellError, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_parse_error, report_shell_error,
};
use nu_std::load_standard_library;
use std::{
    collections::HashMap,
    io::{self, BufRead, Read, Write},
    sync::Arc,
};

pub trait TestBin {
    fn help(&self) -> &'static str;
    fn run(&self);
}

pub struct EchoEnv;
pub struct EchoEnvStderr;
pub struct EchoEnvStderrFail;
pub struct EchoEnvMixed;
pub struct Cococo;
pub struct Meow;
pub struct Meowb;
pub struct Relay;
pub struct Iecho;
pub struct Fail;
pub struct Nonu;
pub struct Chop;
pub struct Repeater;
pub struct RepeatBytes;
pub struct NuRepl;
pub struct InputBytesLength;

impl TestBin for EchoEnv {
    fn help(&self) -> &'static str {
        "Echo's value of env keys from args(e.g: nu --testbin echo_env FOO BAR)"
    }

    fn run(&self) {
        echo_env(true)
    }
}

impl TestBin for EchoEnvStderr {
    fn help(&self) -> &'static str {
        "Echo's value of env keys from args to stderr(e.g: nu --testbin echo_env_stderr FOO BAR)"
    }

    fn run(&self) {
        echo_env(false)
    }
}

impl TestBin for EchoEnvStderrFail {
    fn help(&self) -> &'static str {
        "Echo's value of env keys from args to stderr, and exit with failure(e.g: nu --testbin echo_env_stderr_fail FOO BAR)"
    }

    fn run(&self) {
        echo_env(false);
        fail(1);
    }
}

impl TestBin for EchoEnvMixed {
    fn help(&self) -> &'static str {
        "Mix echo of env keys from input(e.g: nu --testbin echo_env_mixed out-err FOO BAR; nu --testbin echo_env_mixed err-out FOO BAR)"
    }

    fn run(&self) {
        let args = args();
        let args = &args[1..];

        if args.len() != 3 {
            panic!(
                r#"Usage examples:
* nu --testbin echo_env_mixed out-err FOO BAR
* nu --testbin echo_env_mixed err-out FOO BAR"#
            )
        }
        match args[0].as_str() {
            "out-err" => {
                let (out_arg, err_arg) = (&args[1], &args[2]);
                echo_one_env(out_arg, true);
                echo_one_env(err_arg, false);
            }
            "err-out" => {
                let (err_arg, out_arg) = (&args[1], &args[2]);
                echo_one_env(err_arg, false);
                echo_one_env(out_arg, true);
            }
            _ => panic!("The mixed type must be `out_err`, `err_out`"),
        }
    }
}

impl TestBin for Cococo {
    fn help(&self) -> &'static str {
        "Cross platform echo using println!()(e.g: nu --testbin cococo a b c)"
    }

    fn run(&self) {
        let args: Vec<String> = args();

        if args.len() > 1 {
            // Write back out all the arguments passed
            // if given at least 1 instead of chickens
            // speaking co co co.
            println!("{}", &args[1..].join(" "));
        } else {
            println!("cococo");
        }
    }
}

impl TestBin for Meow {
    fn help(&self) -> &'static str {
        "Cross platform cat (open a file, print the contents) using read_to_string and println!()(e.g: nu --testbin meow file.txt)"
    }

    fn run(&self) {
        let args: Vec<String> = args();

        for arg in args.iter().skip(1) {
            let contents = std::fs::read_to_string(arg).expect("Expected a filepath");
            println!("{contents}");
        }
    }
}

impl TestBin for Meowb {
    fn help(&self) -> &'static str {
        "Cross platform cat (open a file, print the contents) using read() and write_all() / binary(e.g: nu --testbin meowb sample.db)"
    }

    fn run(&self) {
        let args: Vec<String> = args();

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for arg in args.iter().skip(1) {
            let buf = std::fs::read(arg).expect("Expected a filepath");
            handle.write_all(&buf).expect("failed to write to stdout");
        }
    }
}

impl TestBin for Relay {
    fn help(&self) -> &'static str {
        "Relays anything received on stdin to stdout(e.g: 0x[beef] | nu --testbin relay)"
    }

    fn run(&self) {
        io::copy(&mut io::stdin().lock(), &mut io::stdout().lock())
            .expect("failed to copy stdin to stdout");
    }
}

impl TestBin for Iecho {
    fn help(&self) -> &'static str {
        "Another type of echo that outputs a parameter per line, looping infinitely(e.g: nu --testbin iecho 3)"
    }

    fn run(&self) {
        // println! panics if stdout gets closed, whereas writeln gives us an error
        let mut stdout = io::stdout();
        let _ = args()
            .iter()
            .skip(1)
            .cycle()
            .try_for_each(|v| writeln!(stdout, "{v}"));
    }
}

impl TestBin for Fail {
    fn help(&self) -> &'static str {
        "Exits with failure code <c>, if not given, fail with code 1(e.g: nu --testbin fail 10)"
    }

    fn run(&self) {
        let args: Vec<String> = args();

        let exit_code: i32 = if args.len() > 1 {
            args[1].parse().expect("given exit_code should be a number")
        } else {
            1
        };
        fail(exit_code);
    }
}

impl TestBin for Nonu {
    fn help(&self) -> &'static str {
        "Cross platform echo but concats arguments without space and NO newline(e.g: nu --testbin nonu a b c)"
    }

    fn run(&self) {
        args().iter().skip(1).for_each(|arg| print!("{arg}"));
    }
}

impl TestBin for Chop {
    fn help(&self) -> &'static str {
        "With no parameters, will chop a character off the end of each line"
    }

    fn run(&self) {
        if did_chop_arguments() {
            // we are done and don't care about standard input.
            std::process::exit(0);
        }

        // if no arguments given, chop from standard input and exit.
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for given in stdin.lock().lines().map_while(Result::ok) {
            let chopped = if given.is_empty() {
                &given
            } else {
                let to = given.len() - 1;
                &given[..to]
            };

            if let Err(_e) = writeln!(stdout, "{chopped}") {
                break;
            }
        }

        std::process::exit(0);
    }
}
impl TestBin for Repeater {
    fn help(&self) -> &'static str {
        "Repeat a string or char N times(e.g: nu --testbin repeater a 5)"
    }

    fn run(&self) {
        let mut stdout = io::stdout();
        let args = args();
        let mut args = args.iter().skip(1);
        let letter = args.next().expect("needs a character to iterate");
        let count = args.next().expect("need the number of times to iterate");

        let count: u64 = count.parse().expect("can't convert count to number");

        for _ in 0..count {
            let _ = write!(stdout, "{letter}");
        }
        let _ = stdout.flush();
    }
}

impl TestBin for RepeatBytes {
    fn help(&self) -> &'static str {
        "A version of repeater that can output binary data, even null bytes(e.g: nu --testbin repeat_bytes 003d9fbf 10)"
    }

    fn run(&self) {
        let mut stdout = io::stdout();
        let args = args();
        let mut args = args.iter().skip(1);

        while let (Some(binary), Some(count)) = (args.next(), args.next()) {
            let bytes: Vec<u8> = (0..binary.len())
                .step_by(2)
                .map(|i| {
                    u8::from_str_radix(&binary[i..i + 2], 16)
                        .expect("binary string is valid hexadecimal")
                })
                .collect();
            let count: u64 = count.parse().expect("repeat count must be a number");

            for _ in 0..count {
                stdout
                    .write_all(&bytes)
                    .expect("writing to stdout must not fail");
            }
        }

        let _ = stdout.flush();
    }
}

impl TestBin for NuRepl {
    fn help(&self) -> &'static str {
        "Run a REPL with the given source lines, it must be called with `--testbin=nu_repl`, `--testbin nu_repl` will not work due to argument count logic"
    }

    fn run(&self) {
        nu_repl();
    }
}

impl TestBin for InputBytesLength {
    fn help(&self) -> &'static str {
        "Prints the number of bytes received on stdin(e.g: 0x[deadbeef] | nu --testbin input_bytes_length)"
    }

    fn run(&self) {
        let stdin = io::stdin();
        let count = stdin.lock().bytes().count();

        println!("{count}");
    }
}

/// Echo's value of env keys from args
/// Example: nu --testbin env_echo FOO BAR
/// If it it's not present echo's nothing
pub fn echo_env(to_stdout: bool) {
    let args = args();
    for arg in args {
        echo_one_env(&arg, to_stdout)
    }
}

fn echo_one_env(arg: &str, to_stdout: bool) {
    if let Ok(v) = std::env::var(arg) {
        if to_stdout {
            println!("{v}");
        } else {
            eprintln!("{v}");
        }
    }
}

pub fn fail(exit_code: i32) {
    std::process::exit(exit_code);
}

fn outcome_err(engine_state: &EngineState, error: &ShellError) -> ! {
    report_shell_error(engine_state, error);
    std::process::exit(1);
}

fn outcome_ok(msg: String) -> ! {
    println!("{msg}");
    std::process::exit(0);
}

/// Generate a minimal engine state with just `nu-cmd-lang`, `nu-command`, and `nu-cli` commands.
fn get_engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    nu_cli::add_cli_context(engine_state)
}

pub fn nu_repl() {
    //cwd: &str, source_lines: &[&str]) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    let source_lines = args();

    let mut engine_state = get_engine_state();
    let mut top_stack = Arc::new(Stack::new());

    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
    engine_state.add_env_var("PATH".into(), Value::test_string(""));

    let mut last_output = String::new();

    load_standard_library(&mut engine_state).expect("Could not load the standard library.");

    for (i, line) in source_lines.iter().enumerate() {
        let mut stack = Stack::with_parent(top_stack.clone());

        // Before doing anything, merge the environment from the previous REPL iteration into the
        // permanent state.
        if let Err(err) = engine_state.merge_env(&mut stack) {
            outcome_err(&engine_state, &err);
        }

        // Check for pre_prompt hook
        let hook = engine_state.get_config().hooks.pre_prompt.clone();
        if let Err(err) = eval_hooks(&mut engine_state, &mut stack, vec![], &hook, "pre_prompt") {
            outcome_err(&engine_state, &err);
        }

        // Check for env change hook
        if let Err(err) = eval_env_change_hook(
            &engine_state.get_config().hooks.env_change.clone(),
            &mut engine_state,
            &mut stack,
        ) {
            outcome_err(&engine_state, &err);
        }

        // Check for pre_execution hook

        engine_state
            .repl_state
            .lock()
            .expect("repl state mutex")
            .buffer = line.to_string();

        let hook = engine_state.get_config().hooks.pre_execution.clone();
        if let Err(err) = eval_hooks(
            &mut engine_state,
            &mut stack,
            vec![],
            &hook,
            "pre_execution",
        ) {
            outcome_err(&engine_state, &err);
        }

        // Eval the REPL line
        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block = parse(
                &mut working_set,
                Some(&format!("line{i}")),
                line.as_bytes(),
                false,
            );

            if let Some(err) = working_set.parse_errors.first() {
                report_parse_error(&working_set, err);
                std::process::exit(1);
            }
            (block, working_set.render())
        };

        if let Err(err) = engine_state.merge_delta(delta) {
            outcome_err(&engine_state, &err);
        }

        let input = PipelineData::empty();
        let config = engine_state.get_config();

        {
            let stack = &mut stack.start_collect_value();
            match eval_block::<WithoutDebug>(&engine_state, stack, &block, input).map(|p| p.body) {
                Ok(pipeline_data) => match pipeline_data.collect_string("", config) {
                    Ok(s) => last_output = s,
                    Err(err) => outcome_err(&engine_state, &err),
                },
                Err(err) => outcome_err(&engine_state, &err),
            }
        }

        if let Some(cwd) = stack.get_env_var(&engine_state, "PWD") {
            let path = cwd
                .coerce_str()
                .unwrap_or_else(|err| outcome_err(&engine_state, &err));
            let _ = std::env::set_current_dir(path.as_ref());
            engine_state.add_env_var("PWD".into(), cwd.clone());
        }
        top_stack = Arc::new(Stack::with_changes_from_child(top_stack, stack));
    }

    outcome_ok(last_output)
}

fn did_chop_arguments() -> bool {
    let args: Vec<String> = args();

    if args.len() > 1 {
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            let chopped = if arg.is_empty() {
                arg
            } else {
                let to = arg.len() - 1;
                &arg[..to]
            };

            println!("{chopped}");
        }

        return true;
    }

    false
}

fn args() -> Vec<String> {
    // skip (--testbin bin_name args)
    std::env::args().skip(2).collect()
}

pub fn show_help(dispatcher: &std::collections::HashMap<String, Box<dyn TestBin>>) {
    println!("Usage: nu --testbin <bin>\n<bin>:");
    let mut names = dispatcher.keys().collect::<Vec<_>>();
    names.sort();
    for n in names {
        let test_bin = dispatcher.get(n).expect("Test bin should exist");
        println!("{n} -> {}", test_bin.help())
    }
}

/// Create a new testbin dispatcher, which is useful to guide the testbin to run.
pub fn new_testbin_dispatcher() -> HashMap<String, Box<dyn TestBin>> {
    let mut dispatcher: HashMap<String, Box<dyn TestBin>> = HashMap::new();
    dispatcher.insert("echo_env".to_string(), Box::new(EchoEnv));
    dispatcher.insert("echo_env_stderr".to_string(), Box::new(EchoEnvStderr));
    dispatcher.insert(
        "echo_env_stderr_fail".to_string(),
        Box::new(EchoEnvStderrFail),
    );
    dispatcher.insert("echo_env_mixed".to_string(), Box::new(EchoEnvMixed));
    dispatcher.insert("cococo".to_string(), Box::new(Cococo));
    dispatcher.insert("meow".to_string(), Box::new(Meow));
    dispatcher.insert("meowb".to_string(), Box::new(Meowb));
    dispatcher.insert("relay".to_string(), Box::new(Relay));
    dispatcher.insert("iecho".to_string(), Box::new(Iecho));
    dispatcher.insert("fail".to_string(), Box::new(Fail));
    dispatcher.insert("nonu".to_string(), Box::new(Nonu));
    dispatcher.insert("chop".to_string(), Box::new(Chop));
    dispatcher.insert("repeater".to_string(), Box::new(Repeater));
    dispatcher.insert("repeat_bytes".to_string(), Box::new(RepeatBytes));
    dispatcher.insert("nu_repl".to_string(), Box::new(NuRepl));
    dispatcher.insert("input_bytes_length".to_string(), Box::new(InputBytesLength));
    dispatcher
}
