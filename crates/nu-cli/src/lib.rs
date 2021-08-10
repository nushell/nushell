mod default_context;
mod errors;
mod syntax_highlight;

pub use default_context::create_default_context;
pub use errors::{report_parsing_error, report_shell_error};
pub use syntax_highlight::NuHighlighter;
