pub mod commands;
pub mod fs;
pub mod locale_override;
pub mod macros;
pub mod playground;

pub struct Outcome {
    pub out: String,
    pub err: String,
}

#[cfg(windows)]
pub const NATIVE_PATH_ENV_VAR: &str = "Path";
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_VAR: &str = "PATH";

#[cfg(windows)]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';

impl Outcome {
    pub fn new(out: String, err: String) -> Outcome {
        Outcome { out, err }
    }
}

pub fn pipeline(commands: &str) -> String {
    commands
        .trim()
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim_end()
        .to_string()
}

pub fn nu_repl_code(source_lines: &[&str]) -> String {
    let mut out = String::from("nu --testbin=nu_repl [ ");

    for line in source_lines.iter() {
        // convert each "line" to really be a single line to prevent nu! macro joining the newlines
        // with ';'
        let line = pipeline(line);

        out.push('`');
        out.push_str(&line);
        out.push('`');
        out.push(' ');
    }

    out.push(']');

    out
}

pub fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

#[cfg(test)]
mod tests {
    use super::pipeline;

    #[test]
    fn constructs_a_pipeline() {
        let actual = pipeline(
            r#"
                open los_tres_amigos.txt
                | from-csv
                | get rusty_luck
                | into int
                | math sum
                | echo "$it"
            "#,
        );

        assert_eq!(
            actual,
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | into int | math sum | echo "$it""#
        );
    }
}
