use std::collections::HashMap;

/// Return map of <deprecated_command_name, new_command_name>
/// This covers simple deprecated commands nicely, but it's not great for deprecating
/// subcommands like `foo bar` where `foo` is still a valid command.
/// For those, it's currently easiest to have a "stub" command that just returns an error.
pub fn deprecated_commands() -> HashMap<String, String> {
    let mut commands = HashMap::new();
    commands.insert("keep".to_string(), "take".to_string());
    commands.insert("match".to_string(), "find".to_string());
    commands.insert("nth".to_string(), "select".to_string());
    commands.insert("pivot".to_string(), "transpose".to_string());
    commands.insert("unalias".to_string(), "hide".to_string());
    commands
}
