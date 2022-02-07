pub mod commands;
pub mod fs;
pub mod macros;
pub mod playground;
<<<<<<< HEAD
pub mod value;
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

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
        .lines()
        .skip(1)
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim_end()
        .to_string()
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
<<<<<<< HEAD
                | str to-int
=======
                | into int
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                | math sum
                | echo "$it"
            "#,
        );

        assert_eq!(
            actual,
<<<<<<< HEAD
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str to-int | math sum | echo "$it""#
=======
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | into int | math sum | echo "$it""#
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        );
    }
}
