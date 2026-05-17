mod from;
mod to;

use nu_plugin::{Plugin, PluginCommand};

use from::eml::FromEml;
use from::ics::FromIcs;
use from::ini::FromIni;
use from::plist::FromPlist;
use from::vcf::FromVcf;
use to::plist::IntoPlist;

pub struct FormatCmdsPlugin;

impl Plugin for FormatCmdsPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(FromEml),
            Box::new(FromIcs),
            Box::new(FromIni),
            Box::new(FromVcf),
            Box::new(FromPlist),
            Box::new(IntoPlist),
        ]
    }
}
