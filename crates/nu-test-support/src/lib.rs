pub mod fs;
pub mod macros;
pub mod playground;

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
                | str --to-int
                | sum
                | echo "$it"
            "#,
        );

        assert_eq!(
            actual,
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str --to-int | sum | echo "$it""#
        );
    }
}
