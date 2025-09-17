// Each const is named after a HistoryItem field, and the value is the field name to be displayed to
// the user (or accept during import).
pub const COMMAND_LINE: &str = "command";

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
mod sqlite_only_fields {
    pub const START_TIMESTAMP: &str = "start_timestamp";
    pub const HOSTNAME: &str = "hostname";
    pub const CWD: &str = "cwd";
    pub const EXIT_STATUS: &str = "exit_status";
    pub const DURATION: &str = "duration";
    pub const SESSION_ID: &str = "session_id";
}

#[cfg_attr(not(feature = "sqlite"), allow(unused))]
pub use sqlite_only_fields::*;
