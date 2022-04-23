mod alias;
mod debug;
mod def;
mod def_env;
mod describe;
mod do_;
mod echo;
mod error_make;
mod export;
mod export_alias;
mod export_def;
mod export_def_env;
mod export_env;
mod export_extern;
mod extern_;
mod for_;
mod help;
mod hide;
mod history;
mod if_;
mod ignore;
mod let_;
mod metadata;
mod module;
mod source;
mod source_bang;
mod tutor;
mod use_;
mod use_bang;
mod version;

pub use alias::Alias;
pub use debug::Debug;
pub use def::Def;
pub use def_env::DefEnv;
pub use describe::Describe;
pub use do_::Do;
pub use echo::Echo;
pub use error_make::ErrorMake;
pub use export::ExportCommand;
pub use export_alias::ExportAlias;
pub use export_def::ExportDef;
pub use export_def_env::ExportDefEnv;
pub use export_env::ExportEnv;
pub use export_extern::ExportExtern;
pub use extern_::Extern;
pub use for_::For;
pub use help::Help;
pub use hide::Hide;
pub use history::History;
pub use if_::If;
pub use ignore::Ignore;
pub use let_::Let;
pub use metadata::Metadata;
pub use module::Module;
pub use source::Source;
pub use source_bang::SourceBang;
pub use tutor::Tutor;
pub use use_::Use;
pub use use_bang::UseBang;
pub use version::Version;

#[cfg(feature = "plugin")]
mod register;

#[cfg(feature = "plugin")]
mod register_bang;

#[cfg(feature = "plugin")]
pub use register::Register;

#[cfg(feature = "plugin")]
pub use register_bang::RegisterBang;
