use std::collections::HashMap;

/// Return map of <removed_command_name, new_command_name>
/// This covers simple removed commands nicely, but it's not great for deprecating
/// subcommands like `foo bar` where `foo` is still a valid command.
/// For those, it's currently easiest to have a "stub" command that just returns an error.
pub fn removed_commands() -> HashMap<String, String> {
    [
        ("fetch".to_string(), "http get".to_string()),
        ("post".to_string(), "http post".to_string()),
        ("benchmark".to_string(), "timeit".to_string()),
    ]
    .into_iter()
    .collect()
}
