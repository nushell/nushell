use std::collections::HashMap;

/// Return map of <deprecated_command_name, new_command_name>
/// This covers simple deprecated commands nicely, but it's not great for deprecating
/// subcommands like `foo bar` where `foo` is still a valid command.
/// For those, it's currently easiest to have a "stub" command that just returns an error.
pub fn deprecated_commands() -> HashMap<String, String> {
    HashMap::from([
        ("keep".to_string(), "take".to_string()),
        ("match".to_string(), "find".to_string()),
        ("nth".to_string(), "select".to_string()),
        ("pivot".to_string(), "transpose".to_string()),
        ("unalias".to_string(), "hide".to_string()),
        ("all?".to_string(), "all".to_string()),
        ("any?".to_string(), "any".to_string()),
        ("empty?".to_string(), "is-empty".to_string()),
    ])
}
