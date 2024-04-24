// `example` command - just suggests to call --help
mod main;

pub use main::Main;

// Basic demos
mod one;
mod three;
mod two;

pub use one::One;
pub use three::Three;
pub use two::Two;

// Engine interface demos
mod config;
mod disable_gc;
mod env;
mod view_span;

pub use config::Config;
pub use disable_gc::DisableGc;
pub use env::Env;
pub use view_span::ViewSpan;

// Stream demos
mod collect_external;
mod for_each;
mod generate;
mod seq;
mod sum;

pub use collect_external::CollectExternal;
pub use for_each::ForEach;
pub use generate::Generate;
pub use seq::Seq;
pub use sum::Sum;
