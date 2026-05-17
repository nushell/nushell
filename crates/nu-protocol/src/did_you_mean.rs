pub fn did_you_mean<'a, 'b, I, S>(possibilities: I, input: &'b str) -> Option<String>
where
    I: IntoIterator<Item = &'a S>,
    S: AsRef<str> + 'a + ?Sized,
{
    let possibilities: Vec<&str> = possibilities.into_iter().map(|s| s.as_ref()).collect();
    let suggestion =
        crate::lev_distance::find_best_match_for_name_with_substrings(&possibilities, input, None)
            .map(|s| s.to_string());
    if let Some(suggestion) = &suggestion
        && suggestion.len() == 1
        && !suggestion.eq_ignore_ascii_case(input)
    {
        return None;
    }
    suggestion
}

#[cfg(test)]
mod tests {

    use super::did_you_mean;

    #[test]
    fn did_you_mean_examples() {
        let all_cases = [
            (
                vec!["a", "b"],
                vec![
                    ("a", Some("a"), ""),
                    ("A", Some("a"), ""),
                    (
                        "c",
                        None,
                        "Not helpful to suggest an arbitrary choice when none are close",
                    ),
                    (
                        "ccccccccccccccccccccccc",
                        None,
                        "Not helpful to suggest an arbitrary choice when none are close",
                    ),
                ],
            ),
            (
                vec!["OS", "PWD", "PWDPWDPWDPWD"],
                vec![
                    (
                        "pwd",
                        Some("PWD"),
                        "Exact case insensitive match yields a match",
                    ),
                    (
                        "pwdpwdpwdpwd",
                        Some("PWDPWDPWDPWD"),
                        "Exact case insensitive match yields a match",
                    ),
                    ("PWF", Some("PWD"), "One-letter typo yields a match"),
                    ("pwf", None, "Case difference plus typo yields no match"),
                    (
                        "Xwdpwdpwdpwd",
                        None,
                        "Case difference plus typo yields no match",
                    ),
                ],
            ),
            (
                vec!["foo", "bar", "baz"],
                vec![
                    ("fox", Some("foo"), ""),
                    ("FOO", Some("foo"), ""),
                    ("FOX", None, ""),
                    (
                        "ccc",
                        None,
                        "Not helpful to suggest an arbitrary choice when none are close",
                    ),
                    (
                        "zzz",
                        None,
                        "'baz' does share a character, but rustc rule is edit distance must be <= 1/3 of the length of the user input",
                    ),
                ],
            ),
            (
                vec!["aaaaaa"],
                vec![
                    (
                        "XXaaaa",
                        Some("aaaaaa"),
                        "Distance of 2 out of 6 chars: close enough to meet rustc's rule",
                    ),
                    (
                        "XXXaaa",
                        None,
                        "Distance of 3 out of 6 chars: not close enough to meet rustc's rule",
                    ),
                    (
                        "XaaaaX",
                        Some("aaaaaa"),
                        "Distance of 2 out of 6 chars: close enough to meet rustc's rule",
                    ),
                    (
                        "XXaaaaXX",
                        None,
                        "Distance of 4 out of 6 chars: not close enough to meet rustc's rule",
                    ),
                ],
            ),
        ];
        for (possibilities, cases) in all_cases {
            for (input, expected_suggestion, discussion) in cases {
                let suggestion = did_you_mean(&possibilities, input);
                assert_eq!(
                    suggestion.as_deref(),
                    expected_suggestion,
                    "Expected the following reasoning to hold but it did not: '{discussion}'"
                );
            }
        }
    }
}
