use chrono::Local;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};
use nu_utils::{get_default_config, get_default_env};
use std::io::Write;

#[derive(Clone)]
pub struct ConfigReset;

impl Command for ConfigReset {
    fn name(&self) -> &str {
        "config reset"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("nu", "reset only nu config, config.nu", Some('n'))
            .switch("env", "reset only env config, env.nu", Some('e'))
            .switch("without-backup", "do not make a backup", Some('w'))
            .category(Category::Env)
    }

    fn usage(&self) -> &str {
        "Reset nushell environment configurations to default, and saves old config files in the config location as oldconfig.nu and oldenv.nu"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "reset nushell configuration files",
            example: "config reset",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let only_nu = call.has_flag("nu");
        let only_env = call.has_flag("env");
        let no_backup = call.has_flag("without-backup");
        let span = call.head;
        let mut config_path = match nu_path::config_dir() {
            Some(path) => path,
            None => {
                return Err(ShellError::GenericError(
                    "Could not find config path".to_string(),
                    "Could not find config path".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };
        config_path.push("nushell");
        if !only_env {
            let mut nu_config = config_path.clone();
            nu_config.push("config.nu");
            let config_file = get_default_config();
            if !no_backup {
                let mut backup_path = config_path.clone();
                backup_path.push(format!(
                    "oldconfig-{}.nu",
                    Local::now().format("%F-%H-%M-%S"),
                ));
                if std::fs::rename(nu_config.clone(), backup_path).is_err() {
                    return Err(ShellError::FileNotFoundCustom(
                        "config.nu could not be backed up".into(),
                        span,
                    ));
                }
            }
            if let Ok(mut file) = std::fs::File::create(nu_config) {
                if writeln!(&mut file, "{}", config_file).is_err() {
                    return Err(ShellError::FileNotFoundCustom(
                        "config.nu could not be written to".into(),
                        span,
                    ));
                }
            }
        }
        if !only_nu {
            let mut env_config = config_path.clone();
            env_config.push("env.nu");
            let config_file = get_default_env();
            if !no_backup {
                let mut backup_path = config_path.clone();
                backup_path.push(format!("oldenv-{}.nu", Local::now().format("%F-%H-%M-%S"),));
                if std::fs::rename(env_config.clone(), backup_path).is_err() {
                    return Err(ShellError::FileNotFoundCustom(
                        "env.nu could not be backed up".into(),
                        span,
                    ));
                }
            }
            if let Ok(mut file) = std::fs::File::create(env_config) {
                if writeln!(&mut file, "{}", config_file).is_err() {
                    return Err(ShellError::FileNotFoundCustom(
                        "env.nu could not be written to".into(),
                        span,
                    ));
                }
            }
        }
        Ok(PipelineData::new(span))
    }
}
