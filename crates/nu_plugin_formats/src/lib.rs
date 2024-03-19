mod from;

use nu_plugin::{Plugin, PluginCommand};

pub use from::eml::FromEml;
pub use from::ics::FromIcs;
pub use from::ini::FromIni;
pub use from::vcf::FromVcf;

pub struct FromCmds;

impl Plugin for FromCmds {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(FromEml),
            Box::new(FromIcs),
            Box::new(FromIni),
            Box::new(FromVcf),
        ]
    }
}
