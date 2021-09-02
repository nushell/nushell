mod eval;
mod state;

pub use eval::{eval_block, eval_expression, eval_operator};
pub use state::{Stack, State};
