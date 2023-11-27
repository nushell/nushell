mod create;
mod delete;
mod insert;
mod open;
mod reset;
mod stor_;
mod update;

pub use create::StorCreate;
pub use delete::StorDelete;
pub use insert::StorInsert;
pub use open::StorOpen;
pub use reset::StorReset;
pub use stor_::Stor;
pub use update::StorUpdate;
