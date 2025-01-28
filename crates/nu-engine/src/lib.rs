#![doc = include_str!("../README.md")]
mod call_ext;
mod closure_eval;
pub mod column;
pub mod command_prelude;
mod compile;
pub mod documentation;
pub mod env;
mod eval;
mod eval_helpers;
mod eval_ir;
mod glob_from;
pub mod scope;

pub use call_ext::CallExt;
pub use closure_eval::*;
pub use column::get_columns;
pub use compile::compile;
pub use documentation::get_full_help;
pub use env::*;
pub use eval::{
    eval_block, eval_block_with_early_return, eval_call, eval_expression,
    eval_expression_with_input, eval_subexpression, eval_variable, is_automatic_env_var,
    redirect_env,
};
pub use eval_helpers::*;
pub use eval_ir::eval_ir_block;
pub use glob_from::glob_from;
