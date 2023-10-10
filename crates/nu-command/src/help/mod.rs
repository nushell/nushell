mod help_;
mod help_aliases;
mod help_commands;
mod help_escapes;
mod help_externs;
mod help_modules;
mod help_operators;

pub use help_::Help;
pub use help_aliases::HelpAliases;
pub use help_commands::HelpCommands;
pub use help_escapes::HelpEscapes;
pub use help_externs::HelpExterns;
pub use help_modules::HelpModules;
pub use help_operators::HelpOperators;

pub(crate) use help_::{highlight_search_in_table, highlight_search_string};
pub(crate) use help_aliases::help_aliases;
pub(crate) use help_commands::help_commands;
pub(crate) use help_modules::help_modules;
