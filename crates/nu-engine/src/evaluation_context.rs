use crate::env::host::Host;
use crate::evaluate::scope::Scope;
use crate::shell::shell_manager::ShellManager;
use crate::{command_args::CommandArgs, script};
use crate::{config_holder::ConfigHolder, Command};
use log::trace;
use nu_data::config::{self, NuConfig};
use nu_errors::ShellError;
use nu_protocol::{ConfigPath, NuScript, RunScriptOptions};
use nu_source::Span;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Clone)]
pub struct EvaluationContext {
    pub scope: Scope,
    pub host: Arc<Mutex<Box<dyn Host>>>,
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

    /// Loads config under cfg_path.
    /// If an error occurs while loading the config:
    ///     The config is not loaded
    ///     The error is returned
    /// After successfull loading of the config the startup scripts are run
    /// as normal scripts (Errors are printed out, ...)
    /// After executing the startup scripts, true is returned to indicate successfull loading
    /// of the config
    //
    // The rational here is that, we should not partially load any config
    // that might be damaged. However, startup scripts might fail for various reasons.
    // A failure there is not as crucial as wrong config files.
    //
    // TODO evaluate how users use this func
    // TODO should error on load be printed out?
    pub async fn load_config(&self, cfg_path: &ConfigPath) -> Result<(), ShellError> {
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

        let tag = config::cfg_path_to_scope_tag(cfg_path);

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

        for script in startup_scripts {
            script::run_script(
                NuScript::Content(script),
                &exit_entry_script_options(&cfg_path),
                &self,
            )
            .await;
        }

        Ok(())
    }

    /// Runs all exit_scripts before unloading the config with path of cfg_path
    /// If no config with path of `cfg_path` is present, this method does nothing
    pub async fn unload_config(&self, cfg_path: &ConfigPath) {
        trace!("UnLoading cfg {:?}", cfg_path);

        let tag = config::cfg_path_to_scope_tag(cfg_path);

        //Run exitscripts with scope frame and cfg still applied
        if let Some(scripts) = self.scope.get_exitscripts_of_frame_with_tag(&tag) {
            for script in scripts {
                script::run_script(
                    NuScript::Content(script),
                    &exit_entry_script_options(&cfg_path),
                    self,
                )
                .await;
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
