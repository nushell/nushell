use crate::env::host::Host;
use crate::evaluate::scope::Scope;
use crate::shell::shell_manager::ShellManager;
use crate::whole_stream_command::Command;
use crate::{call_info::UnevaluatedCallInfo, config_holder::ConfigHolder};
use crate::{command_args::CommandArgs, script};
use log::trace;
use nu_data::config::{self, Conf, NuConfig};
use nu_errors::ShellError;
use nu_protocol::{hir, ConfigPath, NuScript, RunScriptOptions};
use nu_source::{Span, Tag};
use nu_stream::{InputStream, OutputStream};
use parking_lot::Mutex;
use std::{path::Path, sync::Arc};
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[derive(Clone)]
pub struct EvaluationContext {
    pub scope: Scope,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub configs: Arc<Mutex<ConfigHolder>>,
    pub shell_manager: ShellManager,

    /// Windows-specific: keep track of previous cwd on each drive
    pub windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl EvaluationContext {
    pub fn from_args(args: &CommandArgs) -> EvaluationContext {
        EvaluationContext {
            scope: args.scope.clone(),
            host: args.host.clone(),
            current_errors: args.current_errors.clone(),
            ctrl_c: args.ctrl_c.clone(),
            configs: args.configs.clone(),
            shell_manager: args.shell_manager.clone(),
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn error(&self, error: ShellError) {
        self.with_errors(|errors| errors.push(error))
    }

    pub fn clear_errors(&self) {
        self.current_errors.lock().clear()
    }

    pub fn get_errors(&self) -> Vec<ShellError> {
        self.current_errors.lock().clone()
    }

    pub fn configure<T>(
        &mut self,
        config: &dyn nu_data::config::Conf,
        block: impl FnOnce(&dyn nu_data::config::Conf, &mut Self) -> T,
    ) {
        block(config, &mut *self);
    }

    pub fn with_host<T>(&self, block: impl FnOnce(&mut dyn Host) -> T) -> T {
        let mut host = self.host.lock();

        block(&mut *host)
    }

    pub fn with_errors<T>(&self, block: impl FnOnce(&mut Vec<ShellError>) -> T) -> T {
        let mut errors = self.current_errors.lock();

        block(&mut *errors)
    }

    pub fn add_commands(&self, commands: Vec<Command>) {
        for command in commands {
            self.scope.add_command(command.name().to_string(), command);
        }
    }

    #[allow(unused)]
    pub(crate) fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }

    pub fn is_command_registered(&self, name: &str) -> bool {
        self.scope.has_command(name)
    }

    pub(crate) fn run_command(
        &self,
        command: Command,
        name_tag: Tag,
        args: hir::Call,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = self.command_args(args, input, name_tag);
        command.run(command_args)
    }

    fn call_info(&self, args: hir::Call, name_tag: Tag) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo { args, name_tag }
    }

    fn command_args(&self, args: hir::Call, input: InputStream, name_tag: Tag) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            ctrl_c: self.ctrl_c.clone(),
            configs: self.configs.clone(),
            current_errors: self.current_errors.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info(args, name_tag),
            scope: self.scope.clone(),
            input,
        }
    }

    /// Loads config under cfg_path.
    /// If an error occurs while loading the config:
    ///     The config is not loaded
    ///     The error is returned
    /// After successful loading of the config the startup scripts are run
    /// as normal scripts (Errors are printed out, ...)
    /// After executing the startup scripts, true is returned to indicate successful loading
    /// of the config
    //
    // The rational here is that, we should not partially load any config
    // that might be damaged. However, startup scripts might fail for various reasons.
    // A failure there is not as crucial as wrong config files.
    pub fn load_config(&self, cfg_path: &ConfigPath) -> Result<(), ShellError> {
        trace!("Loading cfg {:?}", cfg_path);

        let cfg = NuConfig::load(Some(cfg_path.get_path().clone()))?;
        let exit_scripts = cfg.exit_scripts()?;
        let startup_scripts = cfg.startup_scripts()?;
        let cfg_paths = cfg.path()?;

        let joined_paths = cfg_paths
            .map(|mut cfg_paths| {
                //existing paths are prepended to path
                if let Some(env_paths) = self.scope.get_env("PATH") {
                    let mut env_paths = std::env::split_paths(&env_paths).collect::<Vec<_>>();
                    //No duplicates! Remove env_paths already existing in cfg_paths
                    env_paths.retain(|env_path| !cfg_paths.contains(env_path));
                    //env_paths entries are appended at the end
                    //nu config paths have a higher priority
                    cfg_paths.extend(env_paths);
                }
                cfg_paths
            })
            .map(|paths| {
                std::env::join_paths(paths)
                    .map(|s| s.to_string_lossy().to_string())
                    .map_err(|e| {
                        ShellError::labeled_error(
                            &format!("Error while joining paths from config: {:?}", e),
                            "Config path error",
                            Span::unknown(),
                        )
                    })
            })
            .transpose()?;

        let tag = config::cfg_path_to_scope_tag(cfg_path.get_path());

        self.scope.enter_scope_with_tag(tag);
        self.scope.add_env(cfg.env_map());
        if let Some(path) = joined_paths {
            self.scope.add_env_var("PATH", path);
        }
        self.scope.set_exit_scripts(exit_scripts);

        match cfg_path {
            ConfigPath::Global(_) => self.configs.lock().set_global_cfg(cfg),
            ConfigPath::Local(_) => {
                self.configs.lock().add_local_cfg(cfg);
            }
        }

        let options = exit_entry_script_options(&cfg_path);
        for script in startup_scripts {
            script::run_script(NuScript::Content(script), &options, self);
        }

        Ok(())
    }

    /// Reloads config with a path of cfg_path.
    /// If an error occurs while reloading the config:
    ///     The config is not reloaded
    ///     The error is returned
    pub fn reload_config(&self, cfg: &mut NuConfig) -> Result<(), ShellError> {
        trace!("Reloading cfg {:?}", cfg.file_path);

        cfg.reload();

        let exit_scripts = cfg.exit_scripts()?;
        let cfg_paths = cfg.path()?;

        let joined_paths = cfg_paths
            .map(|mut cfg_paths| {
                //existing paths are prepended to path
                if let Some(env_paths) = self.scope.get_env("PATH") {
                    let mut env_paths = std::env::split_paths(&env_paths).collect::<Vec<_>>();
                    //No duplicates! Remove env_paths already existing in cfg_paths
                    env_paths.retain(|env_path| !cfg_paths.contains(env_path));
                    //env_paths entries are appended at the end
                    //nu config paths have a higher priority
                    cfg_paths.extend(env_paths);
                }
                cfg_paths
            })
            .map(|paths| {
                std::env::join_paths(paths)
                    .map(|s| s.to_string_lossy().to_string())
                    .map_err(|e| {
                        ShellError::labeled_error(
                            &format!("Error while joining paths from config: {:?}", e),
                            "Config path error",
                            Span::unknown(),
                        )
                    })
            })
            .transpose()?;

        let tag = config::cfg_path_to_scope_tag(cfg_path);

        self.scope.update_frame_with_tag(&tag, |frame| {
            frame.env = cfg.env_map();
            if let Some(path) = joined_paths {
                frame.env.insert("PATH".to_string(), path);
            }
            frame.exitscripts = exit_scripts;
        })?;

        Ok(())
    }

    /// Runs all exit_scripts before unloading the config with path of cfg_path
    /// If an error occurs while running exit scripts:
    ///     The error is added to `self.current_errors`
    /// If no config with path of `cfg_path` is present, this method does nothing
    pub fn unload_config(&self, cfg_path: &ConfigPath) {
        trace!("UnLoading cfg {:?}", cfg_path);

        let tag = config::cfg_path_to_scope_tag(cfg_path.get_path());

        //Run exitscripts with scope frame and cfg still applied
        if let Some(scripts) = self.scope.get_exitscripts_of_frame_with_tag(&tag) {
            let options = exit_entry_script_options(&cfg_path);
            for script in scripts {
                script::run_script(NuScript::Content(script), &options, self);
            }
        }

        //Unload config
        self.configs.lock().remove_cfg(&cfg_path);
        self.scope.exit_scope_with_tag(&tag);
    }
}

fn exit_entry_script_options(cfg_path: &ConfigPath) -> RunScriptOptions {
    let root = PathBuf::from("/");
    RunScriptOptions::default()
        .with_cwd(
            cfg_path
                .get_path()
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(root),
        )
        .exit_on_error(false)
}
