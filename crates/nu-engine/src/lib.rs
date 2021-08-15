mod eval;
mod state;
mod value;

pub use eval::{eval_block, eval_expression, eval_operator, ShellError};
pub use state::{Stack, State};
pub use value::Value;
