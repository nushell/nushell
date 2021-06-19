mod logger;
mod options;
mod options_parser;

pub use options::{CliOptions, NuScript, Options};
use options_parser::{NuParser, OptionsParser};

use nu_command::{commands::NuSignature as Nu, utils::test_bins as binaries};
use nu_engine::{get_full_help, EvaluationContext};
use nu_errors::ShellError;
use nu_protocol::hir::{Call, Expression, SpannedExpression, Synthetic};
use nu_protocol::{Primitive, UntaggedValue};
use nu_source::{Span, Tag};
use nu_stream::InputStream;

pub struct App {
    parser: Box<dyn OptionsParser>,
    pub options: Options,
}

impl App {
    pub fn new(parser: Box<dyn OptionsParser>, options: Options) -> Self {
        Self { parser, options }
    }

    pub fn run(args: &[String]) -> Result<(), ShellError> {
        let nu = Box::new(NuParser::new());
        let options = Options::default();
        let ui = App::new(nu, options);

        ui.main(args)
    }

    pub fn main(&self, argv: &[String]) -> Result<(), ShellError> {
        let argv = quote_positionals(argv).join(" ");

        if let Err(cause) = self.parse(&argv) {
            self.parser
                .context()
                .host()
                .lock()
                .print_err(cause, &nu_source::Text::from(argv));
            std::process::exit(1);
        }

        if self.help() {
            let context = self.parser.context();
            let stream = nu_stream::OutputStream::one(
                UntaggedValue::string(get_full_help(&Nu, &context.scope))
                    .into_value(nu_source::Tag::unknown()),
            );

            consume(context, stream)?;

            std::process::exit(0);
        }

        if self.version() {
            let context = self.parser.context();

            let stream = nu_command::commands::version(nu_engine::CommandArgs {
                context: context.clone(),
                call_info: nu_engine::UnevaluatedCallInfo {
                    args: Call::new(
                        Box::new(SpannedExpression::new(
                            Expression::Synthetic(Synthetic::String("version".to_string())),
                            Span::unknown(),
                        )),
                        Span::unknown(),
                    ),
                    name_tag: Tag::unknown(),
                },
                input: InputStream::empty(),
            })?;

            let stream = {
                let command = context
                    .get_command("pivot")
                    .expect("could not find version command");

                context.run_command(
                    command,
                    Tag::unknown(),
                    Call::new(
                        Box::new(SpannedExpression::new(
                            Expression::Synthetic(Synthetic::String("pivot".to_string())),
                            Span::unknown(),
                        )),
                        Span::unknown(),
                    ),
                    stream,
                )?
            };

            consume(context, stream)?;
            std::process::exit(0);
        }

        if let Some(bin) = self.testbin() {
            match bin.as_deref() {
                Ok("echo_env") => binaries::echo_env(),
                Ok("cococo") => binaries::cococo(),
                Ok("meow") => binaries::meow(),
                Ok("iecho") => binaries::iecho(),
                Ok("fail") => binaries::fail(),
                Ok("nonu") => binaries::nonu(),
                Ok("chop") => binaries::chop(),
                Ok("repeater") => binaries::repeater(),
                _ => unreachable!(),
            }

            return Ok(());
        }

        let mut opts = CliOptions::new();

        opts.config = self.config().map(std::ffi::OsString::from);
        opts.stdin = self.takes_stdin();
        opts.save_history = self.save_history();

        use logger::{configure, debug_filters, logger, trace_filters};

        logger(|builder| {
            configure(&self, builder)?;
            trace_filters(&self, builder)?;
            debug_filters(&self, builder)?;

            Ok(())
        })?;

        if let Some(commands) = self.commands() {
            let commands = commands?;
            let script = NuScript::code(&commands)?;
            opts.scripts = vec![script];
            let context = crate::create_default_context(false)?;
            return crate::run_script_file(context, opts);
        }

        if let Some(scripts) = self.scripts() {
            let mut source_files = vec![];
            for script in scripts {
                let script_name = script?;
                let path = std::ffi::OsString::from(&script_name);

                match NuScript::source_file(path.as_os_str()) {
                    Ok(file) => source_files.push(file),
                    Err(_) => {
                        eprintln!("File not found: {}", script_name);
                        return Ok(());
                    }
                }
            }

            for file in source_files {
                let mut opts = opts.clone();
                opts.scripts = vec![file];

                let context = crate::create_default_context(false)?;
                crate::run_script_file(context, opts)?;
            }

            return Ok(());
        }

        let context = crate::create_default_context(true)?;

        if !self.skip_plugins() {
            let _ = crate::register_plugins(&context);
        }

        #[cfg(feature = "rustyline-support")]
        {
            crate::cli(context, opts)?;
        }

        #[cfg(not(feature = "rustyline-support"))]
        {
            println!("Nushell needs the 'rustyline-support' feature for CLI support");
        }

        Ok(())
    }

    pub fn commands(&self) -> Option<Result<String, ShellError>> {
        self.options.get("commands").map(|v| match v.value {
            UntaggedValue::Error(err) => Err(err),
            UntaggedValue::Primitive(Primitive::String(name)) => Ok(name),
            _ => Err(ShellError::untagged_runtime_error("Unsupported option")),
        })
    }

    pub fn help(&self) -> bool {
        self.options
            .get("help")
            .map(|v| matches!(v.as_bool(), Ok(true)))
            .unwrap_or(false)
    }

    pub fn version(&self) -> bool {
        self.options
            .get("version")
            .map(|v| matches!(v.as_bool(), Ok(true)))
            .unwrap_or(false)
    }

    pub fn scripts(&self) -> Option<Vec<Result<String, ShellError>>> {
        self.options.get("args").map(|v| {
            v.table_entries()
                .map(|v| match &v.value {
                    UntaggedValue::Error(err) => Err(err.clone()),
                    UntaggedValue::Primitive(Primitive::FilePath(path)) => {
                        Ok(path.display().to_string())
                    }
                    UntaggedValue::Primitive(Primitive::String(name)) => Ok(name.clone()),
                    _ => Err(ShellError::untagged_runtime_error("Unsupported option")),
                })
                .collect()
        })
    }

    pub fn takes_stdin(&self) -> bool {
        self.options
            .get("stdin")
            .map(|v| matches!(v.as_bool(), Ok(true)))
            .unwrap_or(false)
    }

    pub fn config(&self) -> Option<String> {
        self.options
            .get("config-file")
            .map(|v| v.as_string().expect("not a string"))
    }

    pub fn develop(&self) -> Option<Vec<Result<String, ShellError>>> {
        self.options.get("develop").map(|v| {
            let mut values = vec![];

            match v.value {
                UntaggedValue::Error(err) => values.push(Err(err)),
                UntaggedValue::Primitive(Primitive::String(filters)) => {
                    values.extend(filters.split(',').map(|filter| Ok(filter.to_string())));
                }
                _ => values.push(Err(ShellError::untagged_runtime_error(
                    "Unsupported option",
                ))),
            };

            values
        })
    }

    pub fn debug(&self) -> Option<Vec<Result<String, ShellError>>> {
        self.options.get("debug").map(|v| {
            let mut values = vec![];

            match v.value {
                UntaggedValue::Error(err) => values.push(Err(err)),
                UntaggedValue::Primitive(Primitive::String(filters)) => {
                    values.extend(filters.split(',').map(|filter| Ok(filter.to_string())));
                }
                _ => values.push(Err(ShellError::untagged_runtime_error(
                    "Unsupported option",
                ))),
            };

            values
        })
    }

    pub fn loglevel(&self) -> Option<Result<String, ShellError>> {
        self.options.get("loglevel").map(|v| match v.value {
            UntaggedValue::Error(err) => Err(err),
            UntaggedValue::Primitive(Primitive::String(name)) => Ok(name),
            _ => Err(ShellError::untagged_runtime_error("Unsupported option")),
        })
    }

    pub fn testbin(&self) -> Option<Result<String, ShellError>> {
        self.options.get("testbin").map(|v| match v.value {
            UntaggedValue::Error(err) => Err(err),
            UntaggedValue::Primitive(Primitive::String(name)) => Ok(name),
            _ => Err(ShellError::untagged_runtime_error("Unsupported option")),
        })
    }

    pub fn skip_plugins(&self) -> bool {
        self.options
            .get("skip-plugins")
            .map(|v| matches!(v.as_bool(), Ok(true)))
            .unwrap_or(false)
    }

    pub fn save_history(&self) -> bool {
        self.options
            .get("no-history")
            .map(|v| !matches!(v.as_bool(), Ok(true)))
            .unwrap_or(true)
    }

    pub fn parse(&self, args: &str) -> Result<(), ShellError> {
        self.parser.parse(&args).map(|options| {
            self.options.swap(&options);
        })
    }
}

fn quote_positionals(parameters: &[String]) -> Vec<String> {
    parameters
        .iter()
        .cloned()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{}\"", arg)
            } else {
                arg
            }
        })
        .collect::<Vec<_>>()
}

fn consume(context: &EvaluationContext, stream: InputStream) -> Result<(), ShellError> {
    let autoview_cmd = context
        .get_command("autoview")
        .expect("could not find autoview command");

    let stream = context.run_command(
        autoview_cmd,
        Tag::unknown(),
        Call::new(
            Box::new(SpannedExpression::new(
                Expression::Synthetic(Synthetic::String("autoview".to_string())),
                Span::unknown(),
            )),
            Span::unknown(),
        ),
        stream,
    )?;

    for _ in stream {}

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_app() -> App {
        let parser = Box::new(NuParser::new());
        let options = Options::default();

        App::new(parser, options)
    }

    #[test]
    fn default_options() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu")?;
        assert!(!ui.version());
        assert!(!ui.help());
        assert!(!ui.takes_stdin());
        assert!(ui.save_history());
        assert!(!ui.skip_plugins());
        assert_eq!(ui.config(), None);
        assert_eq!(ui.loglevel(), None);
        assert_eq!(ui.debug(), None);
        assert_eq!(ui.develop(), None);
        assert_eq!(ui.testbin(), None);
        assert_eq!(ui.commands(), None);
        assert_eq!(ui.scripts(), None);
        Ok(())
    }

    #[test]
    fn reports_errors_on_unsupported_flags() -> Result<(), ShellError> {
        let ui = cli_app();

        assert!(ui.parse("nu --coonfig-file /path/to/config.toml").is_err());
        assert!(ui.config().is_none());
        Ok(())
    }

    #[test]
    fn configures_debug_trace_level_with_filters() -> Result<(), ShellError> {
        let ui = cli_app();
        ui.parse("nu --develop=cli,parser")?;
        assert_eq!(ui.develop().unwrap()[0], Ok("cli".to_string()));
        assert_eq!(ui.develop().unwrap()[1], Ok("parser".to_string()));
        Ok(())
    }

    #[test]
    fn configures_debug_level_with_filters() -> Result<(), ShellError> {
        let ui = cli_app();
        ui.parse("nu --debug=cli,run")?;
        assert_eq!(ui.debug().unwrap()[0], Ok("cli".to_string()));
        assert_eq!(ui.debug().unwrap()[1], Ok("run".to_string()));
        Ok(())
    }

    #[test]
    fn can_use_loglevels() -> Result<(), ShellError> {
        for level in &["error", "warn", "info", "debug", "trace"] {
            let ui = cli_app();
            let args = format!("nu --loglevel={}", *level);
            ui.parse(&args)?;
            assert_eq!(ui.loglevel().unwrap(), Ok(level.to_string()));

            let ui = cli_app();
            let args = format!("nu -l {}", *level);
            ui.parse(&args)?;
            assert_eq!(ui.loglevel().unwrap(), Ok(level.to_string()));
        }

        let ui = cli_app();
        ui.parse("nu --loglevel=nada")?;
        assert_eq!(
            ui.loglevel().unwrap(),
            Err(ShellError::untagged_runtime_error("nada is not supported."))
        );

        Ok(())
    }

    #[test]
    fn can_be_passed_nu_scripts() -> Result<(), ShellError> {
        let ui = cli_app();
        ui.parse("nu code.nu bootstrap.nu")?;
        assert_eq!(ui.scripts().unwrap()[0], Ok("code.nu".into()));
        assert_eq!(ui.scripts().unwrap()[1], Ok("bootstrap.nu".into()));
        Ok(())
    }

    #[test]
    fn can_use_test_binaries() -> Result<(), ShellError> {
        for binarie_name in &[
            "echo_env", "cococo", "iecho", "fail", "nonu", "chop", "repeater", "meow",
        ] {
            let ui = cli_app();
            let args = format!("nu --testbin={}", *binarie_name);
            ui.parse(&args)?;
            assert_eq!(ui.testbin().unwrap(), Ok(binarie_name.to_string()));
        }

        let ui = cli_app();
        ui.parse("nu --testbin=andres")?;
        assert_eq!(
            ui.testbin().unwrap(),
            Err(ShellError::untagged_runtime_error(
                "andres is not supported."
            ))
        );

        Ok(())
    }

    #[test]
    fn has_version() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --version")?;
        assert!(ui.version());
        Ok(())
    }

    #[test]
    fn has_help() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --help")?;
        assert!(ui.help());
        Ok(())
    }

    #[test]
    fn can_take_stdin() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --stdin")?;
        assert!(ui.takes_stdin());
        Ok(())
    }

    #[test]
    fn can_opt_to_avoid_saving_history() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --no-history")?;
        assert!(!ui.save_history());
        Ok(())
    }

    #[test]
    fn can_opt_to_skip_plugins() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --skip-plugins")?;
        assert!(ui.skip_plugins());
        Ok(())
    }

    #[test]
    fn understands_commands_need_to_be_run() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu -c \"ls | get name\"")?;
        assert_eq!(ui.commands().unwrap(), Ok(String::from("ls | get name")));

        let ui = cli_app();

        ui.parse("nu -c \"echo 'hola'\"")?;
        assert_eq!(ui.commands().unwrap(), Ok(String::from("echo 'hola'")));
        Ok(())
    }

    #[test]
    fn knows_custom_configurations() -> Result<(), ShellError> {
        let ui = cli_app();

        ui.parse("nu --config-file /path/to/config.toml")?;
        assert_eq!(ui.config().unwrap(), String::from("/path/to/config.toml"));
        Ok(())
    }
}
