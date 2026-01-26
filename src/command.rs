use lexopt::prelude::*;
use nu_experimental as experimental_options;
use nu_parser::escape_for_script_arg;
use nu_protocol::{
    LabeledError, ShellError, Span, Spanned, Value, config::TableMode, did_you_mean,
};
use nu_utils::stdout_write_all_and_flush;
use std::{ffi::OsString, fmt, path::Path};

const HELP_SECTION_COLOR: &str = "\x1b[32m";
const HELP_FLAG_COLOR: &str = "\x1b[36m";
const HELP_TYPE_COLOR: &str = "\x1b[94m";
const HELP_DESC_COLOR: &str = "\x1b[2;39m";
const DEFAULT_COLOR: &str = "\x1b[39m";
const RESET_COLOR: &str = "\x1b[0m";
const TABLE_MODE_VALUES: &[&str] = &[
    "basic",
    "thin",
    "light",
    "compact",
    "with_love",
    "compact_double",
    "default",
    "rounded",
    "reinforced",
    "heavy",
    "none",
    "psql",
    "markdown",
    "dots",
    "restructured",
    "ascii_rounded",
    "basic_compact",
    "single",
    "double",
];
const ERROR_STYLE_VALUES: &[&str] = &["fancy", "plain", "short"];
const LOG_LEVEL_VALUES: &[&str] = &["error", "warn", "info", "debug", "trace"];
const LOG_TARGET_VALUES: &[&str] = &["stdout", "stderr", "mixed", "file"];
const TEST_BIN_VALUES: &[&str] = &[
    "echo_env",
    "echo_env_stderr",
    "echo_env_stderr_fail",
    "echo_env_mixed",
    "cococo",
    "meow",
    "meowb",
    "relay",
    "iecho",
    "fail",
    "nonu",
    "chop",
    "repeater",
    "repeat_bytes",
    "nu_repl",
    "input_bytes_length",
];

// Parsed CLI output with nushell flags and script information.
#[derive(Clone, Debug)]
pub(crate) struct ParsedCli {
    pub(crate) nu: NushellCliArgs,
    pub(crate) script_name: String,
    pub(crate) args_to_script: Vec<String>,
}

// Categories for grouping CLI flags in help output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CliCategory {
    General,
    Startup,
    Config,
    Logging,
    Ide,
    Experimental,
    Plugins,
}

// Expected value types for CLI flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueHint {
    None,
    String,
    Int,
    Path,
    ListString,
    ListPath,
}

// Metadata describing a CLI flag in the lexopt parser.
#[derive(Clone, Copy, Debug)]
struct CliFlag {
    long: &'static str,
    short: Option<char>,
    value: ValueHint,
    description: &'static str,
    category: CliCategory,
    example: &'static str,
}

impl CliFlag {
    // Define a boolean CLI switch.
    const fn switch(
        long: &'static str,
        short: Option<char>,
        description: &'static str,
        category: CliCategory,
        example: &'static str,
    ) -> Self {
        Self {
            long,
            short,
            value: ValueHint::None,
            description,
            category,
            example,
        }
    }

    // Define a CLI option that expects a value.
    const fn value(
        long: &'static str,
        short: Option<char>,
        value: ValueHint,
        description: &'static str,
        category: CliCategory,
        example: &'static str,
    ) -> Self {
        Self {
            long,
            short,
            value,
            description,
            category,
            example,
        }
    }
}

// Primary CLI flag definitions for lexopt parsing and suggestions.
const CLI_FLAGS: &[CliFlag] = &[
    CliFlag::switch(
        "help",
        Some('h'),
        "show this help message",
        CliCategory::General,
        "nu --help",
    ),
    CliFlag::switch(
        "version",
        Some('v'),
        "print the version",
        CliCategory::General,
        "nu --version",
    ),
    CliFlag::switch(
        "interactive",
        Some('i'),
        "start as an interactive shell",
        CliCategory::Startup,
        "nu -i",
    ),
    CliFlag::switch(
        "login",
        Some('l'),
        "start as a login shell",
        CliCategory::Startup,
        "nu -l",
    ),
    CliFlag::value(
        "commands",
        Some('c'),
        ValueHint::String,
        "run the given commands and then exit",
        CliCategory::Startup,
        "nu -c \"print 1\"",
    ),
    CliFlag::value(
        "execute",
        Some('e'),
        ValueHint::String,
        "run the given commands and then enter an interactive shell",
        CliCategory::Startup,
        "nu -e \"print 1\"",
    ),
    CliFlag::value(
        "include-path",
        Some('I'),
        ValueHint::String,
        "set the NU_LIB_DIRS for the given script (delimited by char record_sep ('\x1e'))",
        CliCategory::Config,
        "nu -I scripts",
    ),
    CliFlag::value(
        "table-mode",
        Some('m'),
        ValueHint::String,
        "the table mode to use. rounded is default.",
        CliCategory::Startup,
        "nu -m rounded",
    ),
    CliFlag::value(
        "error-style",
        None,
        ValueHint::String,
        "the error style to use (fancy or plain). default: fancy",
        CliCategory::Startup,
        "nu --error-style plain",
    ),
    CliFlag::switch(
        "no-newline",
        None,
        "print the result for --commands(-c) without a newline",
        CliCategory::Startup,
        "nu --no-newline -c \"print 1\"",
    ),
    CliFlag::switch(
        "no-config-file",
        Some('n'),
        "start with no config file and no env file",
        CliCategory::Config,
        "nu --no-config-file",
    ),
    CliFlag::switch(
        "no-history",
        None,
        "disable reading and writing to command history",
        CliCategory::Config,
        "nu --no-history",
    ),
    CliFlag::switch(
        "no-std-lib",
        None,
        "start with no standard library",
        CliCategory::Config,
        "nu --no-std-lib",
    ),
    CliFlag::value(
        "config",
        None,
        ValueHint::Path,
        "start with an alternate config file",
        CliCategory::Config,
        "nu --config config.nu",
    ),
    CliFlag::value(
        "env-config",
        None,
        ValueHint::Path,
        "start with an alternate environment config file",
        CliCategory::Config,
        "nu --env-config env.nu",
    ),
    CliFlag::value(
        "log-level",
        None,
        ValueHint::String,
        "log level for diagnostic logs (error, warn, info, debug, trace). Off by default",
        CliCategory::Logging,
        "nu --log-level info",
    ),
    CliFlag::value(
        "log-target",
        None,
        ValueHint::String,
        "set the target for the log to output. stdout, stderr(default), mixed or file",
        CliCategory::Logging,
        "nu --log-target stdout",
    ),
    CliFlag::value(
        "log-include",
        None,
        ValueHint::ListString,
        "set the Rust module prefixes to include in the log output. default: [nu]",
        CliCategory::Logging,
        "nu --log-include warn",
    ),
    CliFlag::value(
        "log-exclude",
        None,
        ValueHint::ListString,
        "set the Rust module prefixes to exclude from the log output",
        CliCategory::Logging,
        "nu --log-exclude info",
    ),
    CliFlag::switch(
        "stdin",
        None,
        "redirect standard input to a command (with `-c`) or a script file",
        CliCategory::Startup,
        "nu --stdin -c \"print $in\"",
    ),
    CliFlag::value(
        "testbin",
        None,
        ValueHint::String,
        "run internal test binary",
        CliCategory::Startup,
        "nu --testbin cococo",
    ),
    CliFlag::value(
        "experimental-options",
        None,
        ValueHint::ListString,
        r#"enable or disable experimental options, use "all" to set all active options"#,
        CliCategory::Experimental,
        "nu --experimental-options [example=false]",
    ),
    CliFlag::switch(
        "lsp",
        None,
        "start nu's language server protocol",
        CliCategory::Ide,
        "nu --lsp",
    ),
    CliFlag::value(
        "ide-goto-def",
        None,
        ValueHint::Int,
        "go to the definition of the item at the given position",
        CliCategory::Ide,
        "nu --ide-goto-def 0",
    ),
    CliFlag::value(
        "ide-hover",
        None,
        ValueHint::Int,
        "give information about the item at the given position",
        CliCategory::Ide,
        "nu --ide-hover 0",
    ),
    CliFlag::value(
        "ide-complete",
        None,
        ValueHint::Int,
        "list completions for the item at the given position",
        CliCategory::Ide,
        "nu --ide-complete 0",
    ),
    CliFlag::value(
        "ide-check",
        None,
        ValueHint::Int,
        "run a diagnostic check on the given source and limit number of errors returned to provided number",
        CliCategory::Ide,
        "nu --ide-check 0",
    ),
    CliFlag::switch(
        "ide-ast",
        None,
        "generate the ast on the given source",
        CliCategory::Ide,
        "nu --ide-ast -c \"print 1\"",
    ),
    #[cfg(feature = "plugin")]
    CliFlag::value(
        "plugin-config",
        None,
        ValueHint::Path,
        "start with an alternate plugin registry file",
        CliCategory::Plugins,
        "nu --plugin-config plugins.msgpackz",
    ),
    #[cfg(feature = "plugin")]
    CliFlag::value(
        "plugins",
        None,
        ValueHint::ListPath,
        "list of plugin executable files to load (full paths), separately from the registry file",
        CliCategory::Plugins,
        "nu --plugins /path/nu_plugin_one /path/nu_plugin_two",
    ),
    #[cfg(feature = "mcp")]
    CliFlag::switch(
        "mcp",
        None,
        "start nu's model context protocol server",
        CliCategory::Startup,
        "nu --mcp",
    ),
];

// Container for parsed CLI values before conversion to NushellCliArgs.
#[derive(Clone, Debug, Default)]
struct CliValues {
    redirect_stdin: Option<Spanned<String>>,
    login_shell: Option<Spanned<String>>,
    interactive_shell: Option<Spanned<String>>,
    commands: Option<Spanned<String>>,
    testbin: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    plugin_file: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    plugins: Option<Vec<Spanned<String>>>,
    no_config_file: Option<Spanned<String>>,
    no_history: Option<Spanned<String>>,
    no_std_lib: Option<Spanned<String>>,
    config_file: Option<Spanned<String>>,
    env_file: Option<Spanned<String>>,
    log_level: Option<Spanned<String>>,
    log_target: Option<Spanned<String>>,
    log_include: Option<Vec<Spanned<String>>>,
    log_exclude: Option<Vec<Spanned<String>>>,
    execute: Option<Spanned<String>>,
    table_mode: Option<Value>,
    error_style: Option<Value>,
    no_newline: Option<Spanned<String>>,
    include_path: Option<Spanned<String>>,
    lsp: bool,
    ide_goto_def: Option<Value>,
    ide_hover: Option<Value>,
    ide_complete: Option<Value>,
    ide_check: Option<Value>,
    ide_ast: Option<Spanned<String>>,
    experimental_options: Option<Vec<Spanned<String>>>,
    #[cfg(feature = "mcp")]
    mcp: bool,
}

// Error type for CLI parsing with optional help text.
#[derive(Clone, Debug)]
pub(crate) struct CliError {
    message: String,
    help: Option<String>,
}

impl CliError {
    // Build a new CLI error with a message and label.
    fn new(message: impl Into<String>, _label: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
        }
    }

    // Attach a help message to a CLI error.
    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl fmt::Display for CliError {
    // Render the CLI error message for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

impl From<CliError> for ShellError {
    // Convert a CLI error into a labeled shell error.
    fn from(error: CliError) -> Self {
        let labeled = if let Some(help) = error.help {
            LabeledError::new(error.message).with_help(help)
        } else {
            LabeledError::new(error.message)
        };
        ShellError::from(labeled)
    }
}

// Parse CLI args from the current process environment.
pub(crate) fn parse_cli_args_from_env() -> Result<ParsedCli, CliError> {
    let args = std::env::args_os().collect::<Vec<_>>();
    parse_cli_args(args)
}

// Parse CLI args into nushell options and script details.
pub(crate) fn parse_cli_args(args: Vec<OsString>) -> Result<ParsedCli, CliError> {
    if args.is_empty() {
        return Err(CliError::new(
            "Missing argv0",
            "no executable name provided",
        ));
    }

    prevalidate_short_groups_before_lexopt(&args)?;

    let argv0 = args
        .first()
        .map(|arg| arg.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut parser = lexopt::Parser::from_iter(args);
    let mut cli = CliValues::default();
    let mut script_name = String::new();
    let mut args_to_script = Vec::new();

    if argv0.starts_with('-') {
        cli.login_shell = Some(spanned_true());
    }

    while let Some(arg) = parser.next().map_err(map_lexopt_error)? {
        match arg {
            Short('h') | Long("help") => {
                let help = cli_help_text();
                let _ = std::panic::catch_unwind(move || stdout_write_all_and_flush(help));
                std::process::exit(0);
            }
            Short('v') | Long("version") => {
                let version = env!("CARGO_PKG_VERSION").to_string();
                let _ = std::panic::catch_unwind(move || {
                    stdout_write_all_and_flush(format!("{version}\n"))
                });
                std::process::exit(0);
            }
            Short('i') | Long("interactive") => cli.interactive_shell = Some(spanned_true()),
            Short('l') | Long("login") => cli.login_shell = Some(spanned_true()),
            Short('c') | Long("commands") => {
                let value = parse_string_value(&mut parser, "commands")?;
                cli.commands = Some(spanned_value(value));
            }
            Short('e') | Long("execute") => {
                let value = parse_string_value(&mut parser, "execute")?;
                cli.execute = Some(spanned_value(value));
            }
            Short('I') | Long("include-path") => {
                let value = parse_string_value(&mut parser, "include-path")?;
                cli.include_path = Some(spanned_value(value));
            }
            Short('m') | Long("table-mode") => {
                let value = parse_string_value(&mut parser, "table-mode")?;
                let normalized = value.trim().to_ascii_lowercase();
                match normalized.parse::<TableMode>() {
                    Ok(_) => cli.table_mode = Some(Value::string(value, Span::unknown())),
                    Err(valid) => {
                        let suggestion = did_you_mean(TABLE_MODE_VALUES, &normalized)
                            .map(|item| format!("Did you mean '{item}'?"));
                        let help = suggestion.unwrap_or_else(|| {
                            format!("Valid table modes: {}", TABLE_MODE_VALUES.join(", "))
                        });
                        return Err(CliError::new(
                            "Invalid value for `--table-mode`",
                            format!("expected {valid}"),
                        )
                        .with_help(help));
                    }
                }
            }
            Long("error-style") => {
                let normalized = parse_validated_option(
                    &mut parser,
                    "error-style",
                    ERROR_STYLE_VALUES,
                    "error style",
                )?;
                // Store original case value for error-style
                cli.error_style = Some(Value::string(normalized, Span::unknown()));
            }
            Long("no-newline") => cli.no_newline = Some(spanned_true()),
            Short('n') | Long("no-config-file") => cli.no_config_file = Some(spanned_true()),
            Long("no-history") => cli.no_history = Some(spanned_true()),
            Long("no-std-lib") => cli.no_std_lib = Some(spanned_true()),
            Long("config") => {
                let value = parse_string_value(&mut parser, "config")?;
                cli.config_file = Some(spanned_value(value));
            }
            Long("env-config") => {
                let value = parse_string_value(&mut parser, "env-config")?;
                cli.env_file = Some(spanned_value(value));
            }
            Long("log-level") => {
                let value = parse_validated_option(
                    &mut parser,
                    "log-level",
                    LOG_LEVEL_VALUES,
                    "log level",
                )?;
                cli.log_level = Some(spanned_value(value));
            }
            Long("log-target") => {
                let value = parse_validated_option(
                    &mut parser,
                    "log-target",
                    LOG_TARGET_VALUES,
                    "log target",
                )?;
                cli.log_target = Some(spanned_value(value));
            }
            Long("log-include") => {
                let values = parse_list_values(&mut parser, "log-include")?;
                let parsed = parse_log_filters("log-include", values)?;
                cli.log_include
                    .get_or_insert_with(Vec::new)
                    .extend(parsed.into_iter().map(spanned_value));
            }
            Long("log-exclude") => {
                let values = parse_list_values(&mut parser, "log-exclude")?;
                let parsed = parse_log_filters("log-exclude", values)?;
                cli.log_exclude
                    .get_or_insert_with(Vec::new)
                    .extend(parsed.into_iter().map(spanned_value));
            }
            Long("stdin") => cli.redirect_stdin = Some(spanned_true()),
            Long("testbin") => {
                let normalized =
                    parse_validated_option(&mut parser, "testbin", TEST_BIN_VALUES, "test bin")?;
                cli.testbin = Some(spanned_value(normalized));
            }
            Long("experimental-options") => {
                let values = parse_experimental_options(&mut parser)?;
                cli.experimental_options
                    .get_or_insert_with(Vec::new)
                    .extend(values.into_iter().map(spanned_value));
            }
            Long("lsp") => cli.lsp = true,
            Long("ide-goto-def") => {
                cli.ide_goto_def = Some(parse_ide_int_option(&mut parser, "ide-goto-def")?)
            }
            Long("ide-hover") => {
                cli.ide_hover = Some(parse_ide_int_option(&mut parser, "ide-hover")?)
            }
            Long("ide-complete") => {
                cli.ide_complete = Some(parse_ide_int_option(&mut parser, "ide-complete")?)
            }
            Long("ide-check") => {
                cli.ide_check = Some(parse_ide_int_option(&mut parser, "ide-check")?)
            }
            Long("ide-ast") => cli.ide_ast = Some(spanned_true()),
            #[cfg(feature = "plugin")]
            Long("plugin-config") => {
                let value = parse_string_value(&mut parser, "plugin-config")?;
                cli.plugin_file = Some(spanned_value(value));
            }
            #[cfg(feature = "plugin")]
            Long("plugins") => {
                let values = parse_list_values(&mut parser, "plugins")?;
                let mut parsed = Vec::new();
                for value in values {
                    let trimmed = value.trim();
                    // Skip empty strings and bracket-wrapped empty lists like "[]"
                    if trimmed.is_empty() || trimmed == "[]" {
                        continue;
                    }
                    let path = Path::new(trimmed);
                    let absolute = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        let cwd = std::env::current_dir().map_err(|_| {
                            CliError::new("Invalid value for `--plugins`", "unable to resolve path")
                                .with_help(
                                    "Provide an absolute path or ensure the current directory is available.",
                                )
                        })?;
                        cwd.join(path)
                    };
                    if absolute.is_absolute() {
                        let absolute_str = absolute.display().to_string();
                        parsed.push(spanned_value(absolute_str));
                    } else {
                        return Err(CliError::new(
                            "Invalid value for `--plugins`",
                            "expected full path",
                        )
                        .with_help(
                            "Use an absolute path to the plugin executable, e.g. `nu --plugins /path/nu_plugin_one`."
                        ));
                    }
                }
                // Only set plugins if we actually parsed some valid paths
                if !parsed.is_empty() {
                    cli.plugins.get_or_insert_with(Vec::new).extend(parsed);
                }
            }
            #[cfg(feature = "mcp")]
            Long("mcp") => cli.mcp = true,
            Value(value) => {
                let value = value.string().map_err(|_| {
                    CliError::new("Invalid argument", "argument is not valid unicode")
                        .with_help("Use UTF-8 arguments when calling nushell.")
                })?;
                if script_name.is_empty() {
                    script_name = value;
                    let rest = parser
                        .raw_args()
                        .map_err(map_lexopt_error)?
                        .map(|arg| arg.to_string_lossy().to_string())
                        .map(|arg| escape_for_script_arg(&arg))
                        .collect::<Vec<_>>();
                    args_to_script.extend(rest);
                    break;
                } else {
                    args_to_script.push(escape_for_script_arg(&value));
                }
            }
            Long(name) => return Err(unknown_long_flag(name)),
            Short(short) => return Err(unknown_short_flag(short)),
        }
    }

    Ok(ParsedCli {
        nu: NushellCliArgs {
            redirect_stdin: cli.redirect_stdin,
            login_shell: cli.login_shell,
            interactive_shell: cli.interactive_shell,
            commands: cli.commands,
            testbin: cli.testbin,
            #[cfg(feature = "plugin")]
            plugin_file: cli.plugin_file,
            #[cfg(feature = "plugin")]
            plugins: cli.plugins,
            no_config_file: cli.no_config_file,
            no_history: cli.no_history,
            no_std_lib: cli.no_std_lib,
            config_file: cli.config_file,
            env_file: cli.env_file,
            log_level: cli.log_level,
            log_target: cli.log_target,
            log_include: cli.log_include,
            log_exclude: cli.log_exclude,
            execute: cli.execute,
            table_mode: cli.table_mode,
            error_style: cli.error_style,
            no_newline: cli.no_newline,
            include_path: cli.include_path,
            lsp: cli.lsp,
            ide_goto_def: cli.ide_goto_def,
            ide_hover: cli.ide_hover,
            ide_complete: cli.ide_complete,
            ide_check: cli.ide_check,
            ide_ast: cli.ide_ast,
            experimental_options: cli.experimental_options,
            #[cfg(feature = "mcp")]
            mcp: cli.mcp,
        },
        script_name,
        args_to_script,
    })
}

// Helper to build a spanned boolean-like "true" value.
fn spanned_true() -> Spanned<String> {
    Spanned {
        item: "true".to_string(),
        span: Span::unknown(),
    }
}

// Wrap a string value in a Spanned wrapper with unknown span.
fn spanned_value(value: String) -> Spanned<String> {
    Spanned {
        item: value,
        span: Span::unknown(),
    }
}

// Parse a UTF-8 string value from lexopt for a named option.
fn parse_string_value(parser: &mut lexopt::Parser, name: &str) -> Result<String, CliError> {
    parser
        .value()
        .map_err(map_lexopt_error)?
        .string()
        .map_err(|_| {
            CliError::new(
                format!("Invalid value for `--{name}`"),
                "value is not valid unicode",
            )
            .with_help("Use UTF-8 values when calling nushell.")
        })
}

// Parse and validate a string value against a list of allowed values.
// Returns the normalized (trimmed, lowercase) value if valid.
fn parse_validated_option(
    parser: &mut lexopt::Parser,
    option_name: &str,
    valid_values: &[&str],
    value_description: &str,
) -> Result<String, CliError> {
    let value = parse_string_value(parser, option_name)?;
    let normalized = value.trim().to_ascii_lowercase();
    if valid_values.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        let suggestion =
            did_you_mean(valid_values, &normalized).map(|item| format!("Did you mean '{item}'?"));
        let help = suggestion.unwrap_or_else(|| {
            format!(
                "Valid {} values: {}",
                value_description,
                valid_values.join(", ")
            )
        });
        Err(CliError::new(
            format!("Invalid value for `--{option_name}`"),
            format!("invalid {value_description}"),
        )
        .with_help(help))
    }
}

// Parse and validate an integer value for a named option.
fn parse_int_value(parser: &mut lexopt::Parser, name: &str) -> Result<i64, CliError> {
    let value = parse_string_value(parser, name)?;
    value.parse::<i64>().map_err(|_| {
        CliError::new(
            format!("Invalid value for `--{name}`"),
            format!("expected an integer but got '{value}'"),
        )
        .with_help("Provide a whole number for this option.")
    })
}

// Helper to parse IDE integer options and wrap in Value::int.
fn parse_ide_int_option(parser: &mut lexopt::Parser, name: &str) -> Result<Value, CliError> {
    let value = parse_int_value(parser, name)?;
    Ok(Value::int(value, Span::unknown()))
}

// Parse a list of UTF-8 values for options that accept repeated values.
fn parse_list_values(parser: &mut lexopt::Parser, name: &str) -> Result<Vec<String>, CliError> {
    let values = parser.values().map_err(map_lexopt_error)?;
    let mut parsed = Vec::new();
    for value in values {
        let value = value.string().map_err(|_| {
            CliError::new(
                format!("Invalid value for `--{name}`"),
                "value is not valid unicode",
            )
            .with_help("Use UTF-8 values when calling nushell.")
        })?;
        parsed.push(value);
    }
    Ok(parsed)
}

// Parse experimental options, allowing bracketed and comma-delimited forms.
fn parse_experimental_options(parser: &mut lexopt::Parser) -> Result<Vec<String>, CliError> {
    let values = parse_list_values(parser, "experimental-options")?;
    let mut parsed = Vec::new();
    for value in values {
        let trimmed = value.trim();
        let trimmed = trimmed.strip_prefix('[').unwrap_or(trimmed);
        let trimmed = trimmed.strip_suffix(']').unwrap_or(trimmed);
        if trimmed.contains(',') {
            parsed.extend(
                trimmed
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string),
            );
        } else if !trimmed.is_empty() {
            parsed.push(trimmed.to_string());
        }
    }
    let valid_options = experimental_options::ALL
        .iter()
        .map(|option| option.identifier())
        .collect::<Vec<_>>();
    for option in &parsed {
        let normalized = option.trim();
        if normalized.is_empty() || normalized == "all" {
            continue;
        }
        if let Some((name, _)) = normalized.split_once('=') {
            validate_experimental_option(name.trim(), &valid_options)?;
        } else {
            validate_experimental_option(normalized, &valid_options)?;
        }
    }
    Ok(parsed)
}

// Parse log filters and ensure they match known log levels.
// Supports multiple formats: [error,warn], [error, warn], error warn, etc.
fn parse_log_filters(name: &str, values: Vec<String>) -> Result<Vec<String>, CliError> {
    let mut parsed = Vec::new();

    // Process each value, handling brackets and comma-delimited forms
    for value in values {
        let trimmed = value.trim();
        let trimmed = trimmed.strip_prefix('[').unwrap_or(trimmed);
        let trimmed = trimmed.strip_suffix(']').unwrap_or(trimmed);

        // Split on commas if present, otherwise treat as single value
        if trimmed.contains(',') {
            for item in trimmed.split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    let normalized = item.to_ascii_lowercase();
                    if LOG_LEVEL_VALUES.contains(&normalized.as_str()) {
                        parsed.push(normalized);
                    } else {
                        let suggestion = did_you_mean(LOG_LEVEL_VALUES, &normalized)
                            .map(|item| format!("Did you mean '{item}'?"));
                        let help = suggestion.unwrap_or_else(|| {
                            format!("Valid log levels: {}", LOG_LEVEL_VALUES.join(", "))
                        });
                        return Err(CliError::new(
                            format!("Invalid value for `--{name}`"),
                            "invalid log level",
                        )
                        .with_help(help));
                    }
                }
            }
        } else if !trimmed.is_empty() {
            let normalized = trimmed.to_ascii_lowercase();
            if LOG_LEVEL_VALUES.contains(&normalized.as_str()) {
                parsed.push(normalized);
            } else {
                let suggestion = did_you_mean(LOG_LEVEL_VALUES, &normalized)
                    .map(|item| format!("Did you mean '{item}'?"));
                let help = suggestion.unwrap_or_else(|| {
                    format!("Valid log levels: {}", LOG_LEVEL_VALUES.join(", "))
                });
                return Err(CliError::new(
                    format!("Invalid value for `--{name}`"),
                    "invalid log level",
                )
                .with_help(help));
            }
        }
    }
    Ok(parsed)
}

// Validate an experimental option name against the known list.
fn validate_experimental_option(name: &str, valid_options: &[&str]) -> Result<(), CliError> {
    if valid_options.contains(&name) {
        Ok(())
    } else {
        let suggestion =
            did_you_mean(valid_options, name).map(|item| format!("Did you mean '{item}'?"));
        let help = suggestion
            .unwrap_or_else(|| format!("Valid experimental options: {}", valid_options.join(", ")));
        Err(CliError::new(
            "Invalid value for `--experimental-options`",
            "invalid experimental option",
        )
        .with_help(help))
    }
}

// Helper to generate help text for missing value errors based on option name.
fn missing_value_help(option: &str) -> String {
    match option {
        "-m" | "--table-mode" => format!("Valid table modes: {}", TABLE_MODE_VALUES.join(", ")),
        "--error-style" => format!("Valid error styles: {}", ERROR_STYLE_VALUES.join(", ")),
        "--testbin" => format!("Valid test bins: {}", TEST_BIN_VALUES.join(", ")),
        "--log-level" | "--log-include" | "--log-exclude" => {
            format!("Valid log levels: {}", LOG_LEVEL_VALUES.join(", "))
        }
        "--log-target" => format!("Valid log targets: {}", LOG_TARGET_VALUES.join(", ")),
        "--experimental-options" => {
            let valid_options = experimental_options::ALL
                .iter()
                .map(|opt| opt.identifier())
                .collect::<Vec<_>>();
            format!("Valid experimental options: {}", valid_options.join(", "))
        }
        _ => format!("Provide a value: `{option} <value>` or `{option}=<value>`."),
    }
}

// Map lexopt errors into user-friendly CLI errors.
fn map_lexopt_error(error: lexopt::Error) -> CliError {
    match error {
        lexopt::Error::MissingValue { option } => {
            let (message, help) = if let Some(option) = option {
                let help = missing_value_help(&option);
                (format!("{option} expects a value"), help)
            } else {
                (
                    "Missing value".to_string(),
                    "Provide a value after the option.".to_string(),
                )
            };
            CliError::new(message, "missing value").with_help(help)
        }
        lexopt::Error::UnexpectedValue { option, value } => CliError::new(
            format!("{option} does not take a value"),
            format!("unexpected value '{:?}'", value),
        )
        .with_help(format!("Remove the value or use `{option}` without it.")),
        lexopt::Error::UnexpectedOption(option) => {
            CliError::new(format!("Unknown option '{option}'"), "unknown option")
                .with_help("Use `nu --help` to see available flags.")
        }
        lexopt::Error::UnexpectedArgument(value) => CliError::new(
            format!("Unexpected argument '{:?}'", value),
            "unexpected argument",
        )
        .with_help("Use `nu --help` to see usage."),
        lexopt::Error::ParsingFailed { value, error } => {
            CliError::new(format!("Invalid value '{value}'"), error.to_string())
                .with_help("Check the value format and try again.")
        }
        lexopt::Error::NonUnicodeValue(value) => CliError::new(
            format!("Invalid argument '{:?}'", value),
            "argument is not valid unicode",
        )
        .with_help("Use UTF-8 arguments when calling nushell."),
        lexopt::Error::Custom(error) => CliError::new(error.to_string(), "invalid argument"),
    }
}

// Build an error for unknown long flags with suggestions.
fn unknown_long_flag(name: &str) -> CliError {
    let candidates = CLI_FLAGS
        .iter()
        .map(|flag| format!("--{}", flag.long))
        .collect::<Vec<_>>();
    let suggestion = did_you_mean(&candidates, &format!("--{name}"));
    let help = suggestion
        .map(|s| format!("Did you mean '{s}'?"))
        .unwrap_or_else(|| "Use `nu --help` to see available flags.".to_string());
    CliError::new(format!("Unknown flag '--{name}'"), "unknown flag").with_help(help)
}

// Build an error for unknown short flags with suggestions.
fn unknown_short_flag(short: char) -> CliError {
    let mut candidates = CLI_FLAGS
        .iter()
        .filter_map(|flag| flag.short.map(|s| format!("-{s}")))
        .collect::<Vec<_>>();
    candidates.extend(CLI_FLAGS.iter().map(|flag| format!("--{}", flag.long)));
    let suggestion = did_you_mean(&candidates, &format!("-{short}"));
    let help = suggestion
        .map(|s| format!("Did you mean '{s}'?"))
        .unwrap_or_else(|| "Use `nu --help` to see available flags.".to_string());
    CliError::new(format!("Unknown flag '-{short}'"), "unknown flag").with_help(help)
}

// Validate combined short flags and reject unsupported inline values.
fn prevalidate_short_groups_before_lexopt(args: &[OsString]) -> Result<(), CliError> {
    let mut i = 1; // skip argv0
    let mut skip_next = false;

    while i < args.len() {
        let arg = args[i].to_string_lossy();

        // Skip validation for values following certain flags
        if skip_next {
            skip_next = false;
            i += 1;
            continue;
        }

        // Flags that take command/script strings - stop all validation after these
        if arg == "-c" || arg == "--commands" {
            // Everything after -c/--commands is nushell code, not CLI args
            break;
        }

        // Flags that take a single value - skip validation of their values
        // Note: Multi-value flags (--plugins, --log-include, etc.) are not included here
        // because they consume multiple arguments and the validator can't know how many.
        if arg == "-e"
            || arg == "--execute"
            || arg == "--config"
            || arg == "--env-config"
            || arg == "--plugin-config"
            || arg == "--log-level"
            || arg == "--log-target"
            || arg == "-I"
            || arg == "-m"
            || arg == "--table-mode"
            || arg == "--error-style"
            || arg == "--ide-check"
            || arg == "--ide-goto-def"
            || arg == "--ide-hover"
            || arg == "--ide-complete"
            || arg == "--include-path"
            || arg == "--testbin"
        {
            skip_next = true;
            i += 1;
            continue;
        }

        // Stop validation at script name or positional args
        if !arg.starts_with('-') {
            break;
        }

        if arg == "-" {
            return Err(CliError::new(
                "Invalid short flag",
                "expected a flag after '-'",
            ));
        }

        if arg.starts_with("--") {
            i += 1;
            continue;
        }

        let mut group = arg.trim_start_matches('-');
        if group.is_empty() {
            return Err(CliError::new(
                "Invalid short flag",
                "expected a flag after '-'",
            ));
        }

        let mut inline_value = None;
        if let Some((before, after)) = group.split_once('=') {
            group = before;
            inline_value = Some(after);
        }

        if group.is_empty() {
            return Err(CliError::new(
                "Invalid short flag",
                "expected a flag after '-'",
            ));
        }

        let shorts: Vec<char> = group.chars().collect();

        if let Some(inline) = inline_value {
            let short = shorts.last().copied().unwrap_or('?');
            let expects_value =
                find_short_flag(short).is_some_and(|flag| flag.value != ValueHint::None);
            if inline.is_empty() {
                return Err(
                    CliError::new(format!("`-{short}` expects a value"), "missing value")
                        .with_help("Provide a value after `=` or use `-x <value>`."),
                );
            }
            if !expects_value {
                return Err(CliError::new(
                    format!("`-{short}` does not take a value"),
                    "unexpected value",
                )
                .with_help("Remove the value or use a flag that expects one."));
            }
            if shorts[..shorts.len().saturating_sub(1)]
                .iter()
                .any(|short| {
                    find_short_flag(*short).is_some_and(|flag| flag.value != ValueHint::None)
                })
            {
                return Err(CliError::new(
                    format!("`-{short}` expects a value"),
                    "only the last short flag can take a value",
                )
                .with_help(format!(
                    "Move `-{short}` to the end, then pass a value like `-{short} <value>` or `-{short}=<value>`."
                )));
            }
            i += 1;
            continue;
        }

        if let Some((idx, short)) = shorts.iter().enumerate().find(|(_, short)| {
            find_short_flag(**short).is_some_and(|flag| flag.value != ValueHint::None)
        }) && idx + 1 != shorts.len()
        {
            let trailing_known = shorts[idx + 1..]
                .iter()
                .all(|short| find_short_flag(*short).is_some());
            if trailing_known {
                return Err(CliError::new(
                    format!("`-{short}` expects a value"),
                    "only the last short flag can take a value",
                )
                .with_help(format!(
                    "Move `-{short}` to the end, then pass a value like `-{short} <value>` or `-{short}=<value>`."
                )));
            }
            return Err(CliError::new(
                format!("`-{short}` does not accept inline values"),
                "use a space or `=`",
            )
            .with_help(format!(
                "Use `-{short} <value>` or `-{short}=<value>` instead."
            )));
        }

        i += 1;
    }
    Ok(())
}

// Generate help text with the legacy layout and default help colors.
fn cli_help_text() -> String {
    let mut output = String::new();
    output.push_str("The nushell language and shell.\n\n");
    output.push_str("Usage:\n  nu [options] [script file] [script args]\n\n");
    output.push_str("Options:\n");

    for category in [
        CliCategory::General,
        CliCategory::Startup,
        CliCategory::Config,
        CliCategory::Logging,
        CliCategory::Ide,
        CliCategory::Experimental,
        CliCategory::Plugins,
    ] {
        let flags = CLI_FLAGS.iter().filter(|flag| flag.category == category);
        if flags.clone().next().is_none() {
            continue;
        }
        output.push_str(&format!(
            "\n{HELP_SECTION_COLOR}{}:{RESET_COLOR}\n",
            category_name(category)
        ));
        for flag in flags {
            output.push_str("  ");
            if let Some(short) = flag.short {
                output.push_str(&format!("{HELP_FLAG_COLOR}-{short}{RESET_COLOR}"));
                if !flag.long.is_empty() {
                    output.push_str(&format!("{DEFAULT_COLOR},{RESET_COLOR} "));
                }
            }
            if !flag.long.is_empty() {
                output.push_str(&format!("{HELP_FLAG_COLOR}--{}{RESET_COLOR}", flag.long));
            }
            if flag.value != ValueHint::None {
                output.push_str(&format!(
                    " <{HELP_TYPE_COLOR}{}{RESET_COLOR}>",
                    value_hint(flag.value)
                ));
            }
            output.push_str(&format!(
                "\n      {HELP_DESC_COLOR}{}{RESET_COLOR}\n",
                flag.description
            ));
            output.push_str(&format!(
                "      {HELP_DESC_COLOR}Example: {RESET_COLOR}{}\n",
                flag.example
            ));
        }
    }
    output
}

// Find the CLI flag metadata for a short option.
fn find_short_flag(short: char) -> Option<&'static CliFlag> {
    CLI_FLAGS.iter().find(|flag| flag.short == Some(short))
}

// Convert a flag category into a header label.
fn category_name(category: CliCategory) -> &'static str {
    match category {
        CliCategory::General => "General",
        CliCategory::Startup => "Startup",
        CliCategory::Config => "Configuration",
        CliCategory::Logging => "Logging",
        CliCategory::Ide => "IDE",
        CliCategory::Experimental => "Experimental",
        CliCategory::Plugins => "Plugins",
    }
}

// Convert value hint metadata into a display string.
fn value_hint(value: ValueHint) -> &'static str {
    match value {
        ValueHint::None => "",
        ValueHint::String => "string",
        ValueHint::Int => "int",
        ValueHint::Path => "path",
        ValueHint::ListString => "string...",
        ValueHint::ListPath => "path...",
    }
}

// Parsed Nushell CLI arguments used by main and run paths.
#[derive(Clone, Debug)]
pub(crate) struct NushellCliArgs {
    pub(crate) redirect_stdin: Option<Spanned<String>>,
    pub(crate) login_shell: Option<Spanned<String>>,
    pub(crate) interactive_shell: Option<Spanned<String>>,
    pub(crate) commands: Option<Spanned<String>>,
    pub(crate) testbin: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    pub(crate) plugin_file: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    pub(crate) plugins: Option<Vec<Spanned<String>>>,
    pub(crate) no_config_file: Option<Spanned<String>>,
    pub(crate) no_history: Option<Spanned<String>>,
    pub(crate) no_std_lib: Option<Spanned<String>>,
    pub(crate) config_file: Option<Spanned<String>>,
    pub(crate) env_file: Option<Spanned<String>>,
    pub(crate) log_level: Option<Spanned<String>>,
    pub(crate) log_target: Option<Spanned<String>>,
    pub(crate) log_include: Option<Vec<Spanned<String>>>,
    pub(crate) log_exclude: Option<Vec<Spanned<String>>>,
    pub(crate) execute: Option<Spanned<String>>,
    pub(crate) table_mode: Option<Value>,
    pub(crate) error_style: Option<Value>,
    pub(crate) no_newline: Option<Spanned<String>>,
    pub(crate) include_path: Option<Spanned<String>>,
    pub(crate) lsp: bool,
    pub(crate) ide_goto_def: Option<Value>,
    pub(crate) ide_hover: Option<Value>,
    pub(crate) ide_complete: Option<Value>,
    pub(crate) ide_check: Option<Value>,
    pub(crate) ide_ast: Option<Spanned<String>>,
    pub(crate) experimental_options: Option<Vec<Spanned<String>>>,
    #[cfg(feature = "mcp")]
    pub(crate) mcp: bool,
}
