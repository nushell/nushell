use nu_engine::documentation::get_flags_section;
use nu_protocol::{engine::EngineState, levenshtein_distance};
use nu_utils::IgnoreCaseExt;
use reedline::{Completer, Suggestion};
use std::{fmt::Write, sync::Arc};

pub struct NuHelpCompleter(Arc<EngineState>);

impl NuHelpCompleter {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self(engine_state)
    }

    fn completion_helper(&self, line: &str, pos: usize) -> Vec<Suggestion> {
        let folded_line = line.to_folded_case();

        let mut commands = self
            .0
            .get_decls_sorted(false)
            .into_iter()
            .filter_map(|(_, decl_id)| {
                let decl = self.0.get_decl(decl_id);
                (decl.name().to_folded_case().contains(&folded_line)
                    || decl.usage().to_folded_case().contains(&folded_line)
                    || decl
                        .search_terms()
                        .into_iter()
                        .any(|term| term.to_folded_case().contains(&folded_line))
                    || decl.extra_usage().to_folded_case().contains(&folded_line))
                .then_some(decl)
            })
            .collect::<Vec<_>>();

        commands.sort_by_cached_key(|decl| levenshtein_distance(line, decl.name()));

        commands
            .into_iter()
            .map(|decl| {
                let mut long_desc = String::new();

                let usage = decl.usage();
                if !usage.is_empty() {
                    long_desc.push_str(usage);
                    long_desc.push_str("\r\n\r\n");
                }

                let extra_usage = decl.extra_usage();
                if !extra_usage.is_empty() {
                    long_desc.push_str(extra_usage);
                    long_desc.push_str("\r\n\r\n");
                }

                let sig = decl.signature();
                let _ = write!(long_desc, "Usage:\r\n  > {}\r\n", sig.call_signature());

                if !sig.named.is_empty() {
                    long_desc.push_str(&get_flags_section(Some(&*self.0.clone()), &sig, |v| {
                        v.to_parsable_string(", ", &self.0.config)
                    }))
                }

                if !sig.required_positional.is_empty()
                    || !sig.optional_positional.is_empty()
                    || sig.rest_positional.is_some()
                {
                    long_desc.push_str("\r\nParameters:\r\n");
                    for positional in &sig.required_positional {
                        let _ = write!(long_desc, "  {}: {}\r\n", positional.name, positional.desc);
                    }
                    for positional in &sig.optional_positional {
                        let opt_suffix = if let Some(value) = &positional.default_value {
                            format!(
                                " (optional, default: {})",
                                &value.to_parsable_string(", ", &self.0.config),
                            )
                        } else {
                            (" (optional)").to_string()
                        };
                        let _ = write!(
                            long_desc,
                            "  (optional) {}: {}{}\r\n",
                            positional.name, positional.desc, opt_suffix
                        );
                    }

                    if let Some(rest_positional) = &sig.rest_positional {
                        let _ = write!(
                            long_desc,
                            "  ...{}: {}\r\n",
                            rest_positional.name, rest_positional.desc
                        );
                    }
                }

                let extra: Vec<String> = decl
                    .examples()
                    .iter()
                    .map(|example| example.example.replace('\n', "\r\n"))
                    .collect();

                Suggestion {
                    value: decl.name().into(),
                    description: Some(long_desc),
                    style: None,
                    extra: Some(extra),
                    span: reedline::Span {
                        start: pos - line.len(),
                        end: pos,
                    },
                    append_whitespace: false,
                }
            })
            .collect()
    }
}

impl Completer for NuHelpCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        self.completion_helper(line, pos)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("who", 5, 8, &["whoami"])]
    #[case("hash", 1, 5, &["hash", "hash md5", "hash sha256"])]
    #[case("into f", 0, 6, &["into float", "into filesize"])]
    #[case("into nonexistent", 0, 16, &[])]
    fn test_help_completer(
        #[case] line: &str,
        #[case] start: usize,
        #[case] end: usize,
        #[case] expected: &[&str],
    ) {
        let engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let mut completer = NuHelpCompleter::new(engine_state.into());
        let suggestions = completer.complete(line, end);

        assert_eq!(
            expected.len(),
            suggestions.len(),
            "expected {:?}, got {:?}",
            expected,
            suggestions
                .iter()
                .map(|s| s.value.clone())
                .collect::<Vec<_>>()
        );

        for (exp, actual) in expected.iter().zip(suggestions) {
            assert_eq!(exp, &actual.value);
            assert_eq!(reedline::Span::new(start, end), actual.span);
        }
    }
}
