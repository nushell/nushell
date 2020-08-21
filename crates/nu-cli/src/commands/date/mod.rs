pub mod now;
pub mod utc;
pub mod format;

mod utils;

pub use now::Date as DateNow;
pub use utc::Date as DateUTC;
pub use format::Date as DateFormat;