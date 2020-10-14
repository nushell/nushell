pub use nu_data::config::NuConfig;
use std::fmt::Debug;

#[derive(PartialEq, Debug)]
pub enum AutoPivotMode {
    Auto,
    Always,
    Never,
}

impl AutoPivotMode {
    pub fn is_auto(&self) -> bool {
        matches!(self, AutoPivotMode::Auto)
    }

    pub fn is_always(&self) -> bool {
        matches!(self, AutoPivotMode::Always)
    }

    #[allow(unused)]
    pub fn is_never(&self) -> bool {
        matches!(self, AutoPivotMode::Never)
    }
}

pub trait ConfigExtensions: Debug + Send {
    fn pivot_mode(&self) -> AutoPivotMode;
}

pub fn pivot_mode(config: &NuConfig) -> AutoPivotMode {
    let vars = &config.vars;

    if let Some(mode) = vars.get("pivot_mode") {
        let mode = match mode.as_string() {
            Ok(m) if m.to_lowercase() == "auto" => AutoPivotMode::Auto,
            Ok(m) if m.to_lowercase() == "always" => AutoPivotMode::Always,
            Ok(m) if m.to_lowercase() == "never" => AutoPivotMode::Never,
            _ => AutoPivotMode::Never,
        };

        return mode;
    }

    AutoPivotMode::Never
}

impl ConfigExtensions for NuConfig {
    fn pivot_mode(&self) -> AutoPivotMode {
        pivot_mode(self)
    }
}
