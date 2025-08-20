use chrono::Local;
use nu_engine::command_prelude::*;
use nu_utils::{get_scaffold_config, get_scaffold_env};
use std::{io::Write, path::PathBuf};

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
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .category(Category::Env)
    }

    fn description(&self) -> &str {
        "Reset nushell environment configurations to default, and saves old config files in the config location as oldconfig.nu and oldenv.nu."
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let only_nu = call.has_flag(engine_state, stack, "nu")?;
        let only_env = call.has_flag(engine_state, stack, "env")?;
        let no_backup = call.has_flag(engine_state, stack, "without-backup")?;
        let span = call.head;
        let Some(config_path) = nu_path::nu_config_dir() else {
            return Err(ShellError::ConfigDirNotFound { span: call.head });
        };
        if !only_env {
            let mut nu_config = config_path.clone();
            nu_config.push("config.nu");
            let config_file = get_scaffold_config();
            if !no_backup {
                let mut backup_path = config_path.clone();
                backup_path.push(format!(
                    "oldconfig-{}.nu",
                    Local::now().format("%F-%H-%M-%S"),
                ));
                if let Err(err) = std::fs::rename(nu_config.clone(), &backup_path) {
                    return Err(ShellError::Io(IoError::new_with_additional_context(
                        err.not_found_as(NotFound::Directory),
                        span,
                        PathBuf::from(backup_path),
                        "config.nu could not be backed up",
                    )));
                }
            }
            if let Ok(mut file) = std::fs::File::create(&nu_config) {
                if let Err(err) = writeln!(&mut file, "{config_file}") {
                    return Err(ShellError::Io(IoError::new_with_additional_context(
                        err.not_found_as(NotFound::File),
                        span,
                        PathBuf::from(nu_config),
                        "config.nu could not be written to",
                    )));
                }
            }
        }
        if !only_nu {
            let mut env_config = config_path.clone();
            env_config.push("env.nu");
            let config_file = get_scaffold_env();
            if !no_backup {
                let mut backup_path = config_path.clone();
                backup_path.push(format!("oldenv-{}.nu", Local::now().format("%F-%H-%M-%S"),));
                if let Err(err) = std::fs::rename(env_config.clone(), &backup_path) {
                    return Err(ShellError::Io(IoError::new_with_additional_context(
                        err.not_found_as(NotFound::Directory),
                        span,
                        PathBuf::from(backup_path),
                        "env.nu could not be backed up",
                    )));
                }
            }
            if let Ok(mut file) = std::fs::File::create(&env_config) {
                if let Err(err) = writeln!(&mut file, "{config_file}") {
                    return Err(ShellError::Io(IoError::new_with_additional_context(
                        err.not_found_as(NotFound::File),
                        span,
                        PathBuf::from(env_config),
                        "env.nu could not be written to",
                    )));
                }
            }
        }
        Ok(PipelineData::empty())
    }
}
