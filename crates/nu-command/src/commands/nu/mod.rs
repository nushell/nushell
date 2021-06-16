pub mod command;
mod plugin;

pub use command::Command as Nu;
pub use plugin::SubCommand as NuPlugin;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn testbins() -> Vec<String> {
    vec![
        "echo_env", "cococo", "iecho", "fail", "nonu", "chop", "repeater", "meow",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn loglevels() -> Vec<String> {
    vec!["error", "warn", "info", "debug", "trace"]
        .into_iter()
        .map(String::from)
        .collect()
}
