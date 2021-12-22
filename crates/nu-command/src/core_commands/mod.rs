mod alias;
mod debug;
mod def;
mod describe;
mod do_;
mod echo;
mod export;
mod export_def;
mod export_env;
mod for_;
mod help;
mod hide;
mod history;
mod if_;
mod let_;
mod module;
mod source;
mod use_;
mod version;

pub use alias::Alias;
pub use debug::Debug;
pub use def::Def;
pub use describe::Describe;
pub use do_::Do;
pub use echo::Echo;
pub use export::ExportCommand;
pub use export_def::ExportDef;
pub use export_env::ExportEnv;
pub use for_::For;
pub use help::Help;
pub use hide::Hide;
pub use history::History;
pub use if_::If;
pub use let_::Let;
pub use module::Module;
pub use source::Source;
pub use use_::Use;
pub use version::Version;
#[cfg(feature = "plugin")]
mod register;

#[cfg(feature = "plugin")]
pub use register::Register;
