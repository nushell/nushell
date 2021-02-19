pub mod commands;
pub mod fs;
pub mod macros;
pub mod playground;
pub mod value;

pub struct Outcome {
    pub out: String,
    pub err: String,
}

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

    if let Some(paths) = std::env::var_os("PATH") {
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
                | str to-int
                | math sum
                | echo "$it"
            "#,
        );

        assert_eq!(
            actual,
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str to-int | math sum | echo "$it""#
        );
    }
}
