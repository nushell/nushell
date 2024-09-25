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
mod call_decl;
mod config;
mod ctrlc;
mod disable_gc;
mod env;
mod view_span;

pub use call_decl::CallDecl;
pub use config::Config;
pub use ctrlc::Ctrlc;
pub use disable_gc::DisableGc;
pub use env::Env;
pub use view_span::ViewSpan;

// Stream demos
mod collect_bytes;
mod echo;
mod for_each;
mod generate;
mod seq;
mod sum;

pub use collect_bytes::CollectBytes;
pub use echo::Echo;
pub use for_each::ForEach;
pub use generate::Generate;
pub use seq::Seq;
pub use sum::Sum;
