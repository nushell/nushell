use crate::evaluate::envvar::EnvVar;
use crate::evaluate::evaluator::Variable;
use crate::evaluate::scope::{Scope, ScopeFrame};
use crate::shell::palette::ThemedPalette;
use crate::shell::shell_manager::ShellManager;
use crate::whole_stream_command::Command;
use crate::{call_info::UnevaluatedCallInfo, config_holder::ConfigHolder};
use crate::{command_args::CommandArgs, script};
use crate::{env::basic_host::BasicHost, Host};

use nu_data::config::{self, Conf, NuConfig};
use nu_errors::ShellError;
use nu_path::expand_path;
use nu_protocol::{hir, ConfigPath, VariableRegistry};
use nu_source::Spanned;
use nu_source::{Span, Tag};
use nu_stream::InputStream;
use nu_test_support::NATIVE_PATH_ENV_VAR;

use indexmap::IndexMap;
use log::trace;
use parking_lot::Mutex;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::AtomicBool;
use std::{path::Path, sync::Arc};

#[derive(Clone, Default)]
pub struct EngineState {
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub configs: Arc<Mutex<ConfigHolder>>,
    pub shell_manager: ShellManager,

    /// Windows-specific: keep track of previous cwd on each drive
    pub windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
}
#[derive(Clone, Default)]
pub struct EvaluationContext {
    pub scope: Scope,
    pub engine_state: Arc<EngineState>,
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
            engine_state: Arc::new(EngineState {
                host,
                current_errors,
                ctrl_c,
                configs,
                shell_manager,
                windows_drives_previous_cwd,
            }),
        }
    }

    pub fn basic() -> EvaluationContext {
        let scope = Scope::new();
        let host = BasicHost {};
        let env_vars: IndexMap<String, EnvVar> = host
            .vars()
            .iter()
            .cloned()
            .map(|(k, v)| (k, v.into()))
            .collect();
        scope.add_env(env_vars);

        EvaluationContext {
            scope,
            engine_state: Arc::new(EngineState {
                host: Arc::new(parking_lot::Mutex::new(Box::new(host))),
                current_errors: Arc::new(Mutex::new(vec![])),
                ctrl_c: Arc::new(AtomicBool::new(false)),
                configs: Arc::new(Mutex::new(ConfigHolder::new())),
                shell_manager: ShellManager::basic(),
                windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }),
        }
    }

    pub fn error(&self, error: ShellError) {
        self.with_errors(|errors| errors.push(error))
    }

    pub fn host(&self) -> &Arc<parking_lot::Mutex<Box<dyn Host>>> {
        &self.engine_state.host
    }

    pub fn current_errors(&self) -> &Arc<Mutex<Vec<ShellError>>> {
        &self.engine_state.current_errors
    }

    pub fn ctrl_c(&self) -> &Arc<AtomicBool> {
        &self.engine_state.ctrl_c
    }

    pub fn configs(&self) -> &Arc<Mutex<ConfigHolder>> {
        &self.engine_state.configs
    }

    pub fn shell_manager(&self) -> &ShellManager {
        &self.engine_state.shell_manager
    }

    pub fn windows_drives_previous_cwd(
        &self,
    ) -> &Arc<Mutex<std::collections::HashMap<String, String>>> {
        &self.engine_state.windows_drives_previous_cwd
    }

    pub fn clear_errors(&self) {
        self.engine_state.current_errors.lock().clear()
    }

    pub fn get_errors(&self) -> Vec<ShellError> {
        self.engine_state.current_errors.lock().clone()
    }

    pub fn configure<T>(
        &mut self,
        config: &dyn nu_data::config::Conf,
        block: impl FnOnce(&dyn nu_data::config::Conf, &mut Self) -> T,
    ) {
        block(config, &mut *self);
    }

    pub fn with_host<T>(&self, block: impl FnOnce(&mut dyn Host) -> T) -> T {
        let mut host = self.engine_state.host.lock();

        block(&mut *host)
    }

    pub fn with_errors<T>(&self, block: impl FnOnce(&mut Vec<ShellError>) -> T) -> T {
        let mut errors = self.engine_state.current_errors.lock();

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
                std::env::set_var(var, expand_path(val));
                break;
            }
        }
    }

    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }

    pub fn is_command_registered(&self, name: &str) -> bool {
        self.scope.has_command(name)
    }

    pub fn run_command(
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
        let config_env = cfg.env_map();
        let env_vars = config_env
            .into_iter()
            .map(|(k, v)| (k, EnvVar::from(v)))
            .collect();
        self.scope.add_env(env_vars);
        if let Some(path) = joined_paths {
            self.scope.add_env_var(NATIVE_PATH_ENV_VAR, path);
        }
        self.scope.set_exit_scripts(exit_scripts);

        match cfg_path {
            ConfigPath::Global(_) => self.engine_state.configs.lock().set_global_cfg(cfg),
            ConfigPath::Local(_) => {
                self.engine_state.configs.lock().add_local_cfg(cfg);
            }
        }

        // The syntax_theme is really the file stem of a json file i.e.
        // grape.json is the theme file and grape is the file stem and
        // the syntax_theme and grape.json would be located in the same
        // folder as the config.toml

        // Let's open the config
        let global_config = self.engine_state.configs.lock().global_config();
        // Get the root syntax_theme value
        let syntax_theme = global_config.var("syntax_theme");
        // If we have a syntax_theme let's process it
        if let Some(theme_value) = syntax_theme {
            // Append the .json to the syntax_theme to form the file name
            let syntax_theme_filename = format!("{}.json", theme_value.convert_to_string());
            // Load the syntax config json
            let config_file_path = cfg_path.get_path();
            // The syntax file should be in the same location as the config.toml
            let syntax_file_path = if config_file_path.ends_with("config.toml") {
                config_file_path
                    .display()
                    .to_string()
                    .replace("config.toml", &syntax_theme_filename)
            } else {
                "".to_string()
            };
            // if we have a syntax_file_path use it otherwise default
            if Path::new(&syntax_file_path).exists() {
                // eprintln!("Loading syntax file: [{:?}]", syntax_file_path);
                let syntax_theme_file = File::open(syntax_file_path)?;
                let mut reader = BufReader::new(syntax_theme_file);
                let theme = ThemedPalette::new(&mut reader).unwrap_or_default();
                // eprintln!("Theme: [{:?}]", theme);
                self.engine_state.configs.lock().set_syntax_colors(theme);
            } else {
                // If the file was missing, use the default
                self.engine_state
                    .configs
                    .lock()
                    .set_syntax_colors(ThemedPalette::default())
            }
        } else {
            // if there's no syntax_theme, use the default
            self.engine_state
                .configs
                .lock()
                .set_syntax_colors(ThemedPalette::default())
        };

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
        let config_env = cfg.env_map();
        let env_vars = config_env
            .into_iter()
            .map(|(k, v)| (k, EnvVar::from(v)))
            .collect();
        frame.env = env_vars;
        if let Some(path) = joined_paths {
            frame
                .env
                .insert(NATIVE_PATH_ENV_VAR.to_string(), path.into());
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
        self.engine_state.configs.lock().remove_cfg(cfg_path);
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
                        self.engine_state.host.lock().print_err(err, &text);
                    }
                }
            }
        }
    }
}

use itertools::Itertools;

impl VariableRegistry for EvaluationContext {
    fn get_variable(&self, name: &Spanned<&str>) -> Option<nu_protocol::Value> {
        let span = name.span;
        let name = nu_protocol::hir::Expression::variable(name.item.to_string(), name.span);

        let var = Variable::from(&name);

        crate::evaluate::evaluator::evaluate_reference(&var, self, span).ok()
    }

    fn variables(&self) -> Vec<String> {
        Variable::list()
            .into_iter()
            .chain(self.scope.get_variable_names())
            .unique()
            .collect()
    }
}
