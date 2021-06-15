use nu_engine::WholeStreamCommand;
use nu_protocol::{Signature, SyntaxShape};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu")
            .switch("stdin", "stdin", None)
            .switch("skip-plugins", "do not load plugins", None)
            .switch("no-history", "don't save history", None)
            .named("commands", SyntaxShape::String, "Nu commands", Some('c'))
            .named(
                "testbin",
                SyntaxShape::String,
                "BIN: echo_env, cococo, iecho, fail, nonu, chop, repeater, meow",
                None,
            )
            .named("develop", SyntaxShape::String, "trace mode", None)
            .named("debug", SyntaxShape::String, "debug mode", None)
            .named(
                "loglevel",
                SyntaxShape::String,
                "LEVEL: error, warn, info, debug, trace",
                Some('l'),
            )
            .named(
                "config-file",
                SyntaxShape::FilePath,
                "custom configuration source file",
                None,
            )
            .optional("script", SyntaxShape::FilePath, "The Nu script to run")
            .rest(SyntaxShape::String, "Left overs...")
    }

    fn usage(&self) -> &str {
        "Nu"
    }
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
