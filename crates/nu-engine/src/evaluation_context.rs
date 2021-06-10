use crate::evaluate::scope::{Scope, ScopeFrame};
use crate::shell::shell_manager::ShellManager;
use crate::whole_stream_command::Command;
use crate::{call_info::UnevaluatedCallInfo, config_holder::ConfigHolder};
use crate::{command_args::CommandArgs, script};
use crate::{env::basic_host::BasicHost, Host};
use indexmap::IndexMap;
use log::trace;
use nu_data::config::{self, Conf, NuConfig};
use nu_errors::ShellError;
use nu_protocol::{hir, ConfigPath};
use nu_source::{Span, Tag};
use nu_stream::InputStream;
use nu_test_support::NATIVE_PATH_ENV_VAR;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::{path::Path, sync::Arc};

#[derive(Clone, Default)]
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
    pub fn new(
        scope: Scope,
        host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
        current_errors: Arc<Mutex<Vec<ShellError>>>,
        ctrl_c: Arc<AtomicBool>,
        configs: Arc<Mutex<ConfigHolder>>,
        shell_manager: ShellManager,
        windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
    ) -> Self {
        Self {
            scope,
            host,
            current_errors,
            ctrl_c,
            configs,
            shell_manager,
            windows_drives_previous_cwd,
        }
    }

    pub fn basic() -> EvaluationContext {
        let scope = Scope::new();
        let mut host = BasicHost {};
        let env_vars = host.vars().iter().cloned().collect::<IndexMap<_, _>>();
        scope.add_env(env_vars);

        EvaluationContext {
            scope,
            host: Arc::new(parking_lot::Mutex::new(Box::new(host))),
            current_errors: Arc::new(Mutex::new(vec![])),
            ctrl_c: Arc::new(AtomicBool::new(false)),
            configs: Arc::new(Mutex::new(ConfigHolder::new())),
            shell_manager: ShellManager::basic(),
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn from_args(args: &CommandArgs) -> EvaluationContext {
        args.context.clone()
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

    pub fn sync_path_to_env(&self) {
        let env_vars = self.scope.get_env_vars();

        for (var, val) in env_vars {
            if var == NATIVE_PATH_ENV_VAR {
                std::env::set_var(var, val);
                break;
            }
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
    ) -> Result<InputStream, ShellError> {
        let command_args = self.command_args(args, input, name_tag);
        command.run(command_args)
    }

    fn call_info(&self, args: hir::Call, name_tag: Tag) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo { args, name_tag }
    }

    fn command_args(&self, args: hir::Call, input: InputStream, name_tag: Tag) -> CommandArgs {
        CommandArgs {
            context: self.clone(),
            call_info: self.call_info(args, name_tag),
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
                let env_paths = self.scope.get_env(NATIVE_PATH_ENV_VAR);

                if let Some(env_paths) = env_paths {
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
            self.scope.add_env_var(NATIVE_PATH_ENV_VAR, path);
        }
        self.scope.set_exit_scripts(exit_scripts);

        match cfg_path {
            ConfigPath::Global(_) => self.configs.lock().set_global_cfg(cfg),
            ConfigPath::Local(_) => {
                self.configs.lock().add_local_cfg(cfg);
            }
        }

        if !startup_scripts.is_empty() {
            self.run_scripts(startup_scripts, cfg_path.get_path().parent());
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
                let env_paths = self.scope.get_env(NATIVE_PATH_ENV_VAR);

                if let Some(env_paths) = env_paths {
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

        let tag = config::cfg_path_to_scope_tag(&cfg.file_path);
        let mut frame = ScopeFrame::with_tag(tag.clone());

        frame.env = cfg.env_map();
        if let Some(path) = joined_paths {
            frame.env.insert(NATIVE_PATH_ENV_VAR.to_string(), path);
        }
        frame.exitscripts = exit_scripts;

        self.scope.update_frame_with_tag(frame, &tag)?;

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
            self.run_scripts(scripts, cfg_path.get_path().parent());
        }

        //Unload config
        self.configs.lock().remove_cfg(cfg_path);
        self.scope.exit_scope_with_tag(&tag);
    }

    /// Runs scripts with cwd of dir. If dir is None, this method does nothing.
    /// Each error is added to `self.current_errors`
    pub fn run_scripts(&self, scripts: Vec<String>, dir: Option<&Path>) {
        if let Some(dir) = dir {
            for script in scripts {
                match script::run_script_in_dir(script.clone(), dir, self) {
                    Ok(_) => {}
                    Err(e) => {
                        let err = ShellError::untagged_runtime_error(format!(
                            "Err while executing exitscript. Err was\n{:?}",
                            e
                        ));
                        let text = script.into();
                        self.host.lock().print_err(err, &text);
                    }
                }
            }
        }
    }
}
