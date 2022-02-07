use crate::shell::palette::ThemedPalette;
use nu_data::config::NuConfig;
use nu_protocol::ConfigPath;
use std::path::Path;

/// ConfigHolder holds information which configs have been loaded.
#[derive(Clone)]
pub struct ConfigHolder {
    pub global_config: Option<NuConfig>,
    pub local_configs: Vec<NuConfig>,
    pub syntax_config: Option<ThemedPalette>,
}

impl Default for ConfigHolder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigHolder {
    pub fn new() -> ConfigHolder {
        ConfigHolder {
            global_config: None,
            local_configs: vec![],
            syntax_config: None,
        }
    }

    pub fn global_config(&self) -> NuConfig {
        match &self.global_config {
            Some(config) => config.clone(),
            None => NuConfig::default(),
        }
    }

    pub fn syntax_colors(&self) -> ThemedPalette {
        match &self.syntax_config {
            Some(cfg) => cfg.clone(),
            None => ThemedPalette::default(),
        }
    }

    pub fn add_local_cfg(&mut self, cfg: NuConfig) {
        self.local_configs.push(cfg);
    }

    pub fn set_global_cfg(&mut self, cfg: NuConfig) {
        self.global_config = Some(cfg);
    }

    pub fn set_syntax_colors(&mut self, cfg: ThemedPalette) {
        self.syntax_config = Some(cfg);
    }

    pub fn remove_cfg(&mut self, cfg_path: &ConfigPath) {
        match cfg_path {
            ConfigPath::Global(_) => self.global_config = None,
            ConfigPath::Local(p) => self.remove_local_cfg(p),
        }
    }

    fn remove_local_cfg<P: AsRef<Path>>(&mut self, cfg_path: P) {
        // Remove the first loaded local config with specified cfg_path
        if let Some(index) = self
            .local_configs
            .iter()
            .rev()
            .position(|cfg| cfg.file_path == cfg_path.as_ref())
        {
            self.local_configs.remove(index);
        }
    }
}
