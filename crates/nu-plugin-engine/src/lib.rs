//! Provides functionality for running Nushell plugins from a Nushell engine.

mod context;
mod declaration;
mod gc;
mod init;
mod interface;
mod persistent;
mod plugin_custom_value_with_source;
mod process;
mod source;
mod util;

#[cfg(test)]
mod test_util;

pub use context::{PluginExecutionCommandContext, PluginExecutionContext};
pub use declaration::PluginDeclaration;
pub use gc::PluginGc;
pub use init::*;
pub use interface::{PluginInterface, PluginInterfaceManager};
pub use persistent::{GetPlugin, PersistentPlugin};
pub use plugin_custom_value_with_source::{PluginCustomValueWithSource, WithSource};
pub use source::PluginSource;
