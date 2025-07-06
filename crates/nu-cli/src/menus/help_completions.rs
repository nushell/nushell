use nu_engine::documentation::{FormatterValue, HelpStyle, get_flags_section};
use nu_protocol::{Config, engine::EngineState, levenshtein_distance};
use nu_utils::IgnoreCaseExt;
use reedline::{Completer, Suggestion};
use std::{fmt::Write, sync::Arc};

pub struct NuHelpCompleter {
    engine_state: Arc<EngineState>,
    config: Arc<Config>,
}

impl NuHelpCompleter {
    pub fn new(engine_state: Arc<EngineState>, config: Arc<Config>) -> Self {
        Self {
            engine_state,
            config,
        }
    }

    fn completion_helper(&self, line: &str, pos: usize) -> Vec<Suggestion> {
        let folded_line = line.to_folded_case();

        let mut help_style = HelpStyle::default();
        help_style.update_from_config(&self.engine_state, &self.config);

        let mut commands = self
            .engine_state
            .get_decls_sorted(false)
            .into_iter()
            .filter_map(|(_, decl_id)| {
                let decl = self.engine_state.get_decl(decl_id);
                (decl.name().to_folded_case().contains(&folded_line)
                    || decl.description().to_folded_case().contains(&folded_line)
                    || decl
                        .search_terms()
                        .into_iter()
                        .any(|term| term.to_folded_case().contains(&folded_line))
                    || decl
                        .extra_description()
                        .to_folded_case()
                        .contains(&folded_line))
                .then_some(decl)
            })
            .collect::<Vec<_>>();

        commands.sort_by_cached_key(|decl| levenshtein_distance(line, decl.name()));

        commands
            .into_iter()
            .map(|decl| {
                let mut long_desc = String::new();

                let description = decl.description();
                if !description.is_empty() {
                    long_desc.push_str(description);
                    long_desc.push_str("\r\n\r\n");
                }

                let extra_desc = decl.extra_description();
                if !extra_desc.is_empty() {
                    long_desc.push_str(extra_desc);
                    long_desc.push_str("\r\n\r\n");
                }

                let sig = decl.signature();
                let _ = write!(long_desc, "Usage:\r\n  > {}\r\n", sig.call_signature());

                if !sig.named.is_empty() {
                    long_desc.push_str(&get_flags_section(&sig, &help_style, |v| match v {
                        FormatterValue::DefaultValue(value) => {
                            value.to_parsable_string(", ", &self.config)
                        }
                        FormatterValue::CodeString(text) => text.to_string(),
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
                                &value.to_parsable_string(", ", &self.config),
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
                    extra: Some(extra),
                    span: reedline::Span {
                        start: pos - line.len(),
                        end: pos,
                    },
                    ..Suggestion::default()
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
        let config = engine_state.get_config().clone();
        let mut completer = NuHelpCompleter::new(engine_state.into(), config);
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
