use std::{fmt::Write, sync::Arc};

use nu_engine::documentation::get_flags_section;
use nu_protocol::{engine::EngineState, levenshtein_distance};
use nu_utils::IgnoreCaseExt;
use reedline::{Completer, Suggestion};

pub struct NuHelpCompleter(Arc<EngineState>);

impl NuHelpCompleter {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self(engine_state)
    }

    fn completion_helper(&self, line: &str, pos: usize) -> Vec<Suggestion> {
        let full_commands = self.0.get_signatures_with_examples(false);
        let folded_line = line.to_folded_case();

        // Vec<(Signature, Vec<Example>, bool, bool)> {
        let mut commands = full_commands
            .iter()
            .filter(|(sig, _, _, _, _)| {
                sig.name.to_folded_case().contains(&folded_line)
                    || sig.usage.to_folded_case().contains(&folded_line)
                    || sig
                        .search_terms
                        .iter()
                        .any(|term| term.to_folded_case().contains(&folded_line))
                    || sig.extra_usage.to_folded_case().contains(&folded_line)
            })
            .collect::<Vec<_>>();

        commands.sort_by(|(a, _, _, _, _), (b, _, _, _, _)| {
            let a_distance = levenshtein_distance(line, &a.name);
            let b_distance = levenshtein_distance(line, &b.name);
            a_distance.cmp(&b_distance)
        });

        commands
            .into_iter()
            .map(|(sig, examples, _, _, _)| {
                let mut long_desc = String::new();

                let usage = &sig.usage;
                if !usage.is_empty() {
                    long_desc.push_str(usage);
                    long_desc.push_str("\r\n\r\n");
                }

                let extra_usage = &sig.extra_usage;
                if !extra_usage.is_empty() {
                    long_desc.push_str(extra_usage);
                    long_desc.push_str("\r\n\r\n");
                }

                let _ = write!(long_desc, "Usage:\r\n  > {}\r\n", sig.call_signature());

                if !sig.named.is_empty() {
                    long_desc.push_str(&get_flags_section(Some(&*self.0.clone()), sig, |v| {
                        v.into_string_parsable(", ", &self.0.config)
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
                                &value.into_string_parsable(", ", &self.0.config),
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

                let extra: Vec<String> = examples
                    .iter()
                    .map(|example| example.example.replace('\n', "\r\n"))
                    .collect();

                Suggestion {
                    value: sig.name.clone(),
                    description: Some(long_desc),
                    extra: Some(extra),
                    span: reedline::Span {
                        start: pos,
                        end: pos + line.len(),
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
