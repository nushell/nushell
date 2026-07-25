use super::prelude::*;
use crate as nu_protocol;

/// The prompt indicators shown to the right of `$env.PROMPT_COMMAND`.
///
/// These were historically set through the bare `$env.PROMPT_INDICATOR`,
/// `$env.PROMPT_INDICATOR_VI_INSERT`, `$env.PROMPT_INDICATOR_VI_NORMAL` and
/// `$env.PROMPT_MULTILINE_INDICATOR` environment variables. Those still take
/// precedence, but are deprecated in favour of this config section.
#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Shown in emacs mode and whenever no edit mode applies.
    pub indicator: String,
    /// Shown in vi insert mode.
    pub vi_insert: String,
    /// Shown in vi normal mode.
    pub vi_normal: String,
    /// Shown in vi visual mode.
    pub vi_visual: String,
    /// Shown on continuation lines of a multiline entry.
    pub multiline: String,
    /// Overrides applied once a line is submitted and the prompt is redrawn.
    pub transient: TransientPromptConfig,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            indicator: "> ".into(),
            vi_insert: ": ".into(),
            vi_normal: "> ".into(),
            vi_visual: "v ".into(),
            multiline: "::: ".into(),
            transient: TransientPromptConfig::default(),
        }
    }
}

/// Indicators for the transient prompt, which replaces the real one in
/// scrollback once a line is submitted.
///
/// A `null` field keeps whatever the live prompt showed, so only the segments
/// worth condensing need spelling out. These were historically the
/// `$env.TRANSIENT_PROMPT_INDICATOR*` and
/// `$env.TRANSIENT_PROMPT_MULTILINE_INDICATOR` environment variables, which
/// still take precedence but are deprecated.
#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransientPromptConfig {
    /// Replaces [`PromptConfig::indicator`].
    pub indicator: Option<String>,
    /// Replaces [`PromptConfig::vi_insert`].
    pub vi_insert: Option<String>,
    /// Replaces [`PromptConfig::vi_normal`].
    pub vi_normal: Option<String>,
    /// Replaces [`PromptConfig::vi_visual`].
    pub vi_visual: Option<String>,
    /// Replaces [`PromptConfig::multiline`].
    ///
    /// Unlike the others this defaults to empty rather than `null`, matching
    /// the `TRANSIENT_PROMPT_MULTILINE_INDICATOR` that `src/main.rs` used to
    /// seed: dropping the continuation marker keeps submitted multiline
    /// entries copyable out of scrollback.
    pub multiline: Option<String>,
}

impl Default for TransientPromptConfig {
    fn default() -> Self {
        Self {
            indicator: None,
            vi_insert: None,
            vi_normal: None,
            vi_visual: None,
            multiline: Some(String::new()),
        }
    }
}

impl UpdateFromValue for TransientPromptConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let field = match col.as_str() {
                "indicator" => &mut self.indicator,
                "vi_insert" => &mut self.vi_insert,
                "vi_normal" => &mut self.vi_normal,
                "vi_visual" => &mut self.vi_visual,
                "multiline" => &mut self.multiline,
                _ => {
                    errors.unknown_option(path, val);
                    continue;
                }
            };

            // `null` means "keep the live prompt's value" rather than being a
            // type error, so these are the only string keys accepting nothing.
            match val {
                Value::Nothing { .. } => *field = None,
                Value::String { val, .. } => *field = Some(val.clone()),
                _ => errors.type_mismatch(path, Type::custom("string or nothing"), val),
            }
        }
    }
}

impl UpdateFromValue for PromptConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "indicator" => self.indicator.update(val, path, errors),
                "vi_insert" => self.vi_insert.update(val, path, errors),
                "vi_normal" => self.vi_normal.update(val, path, errors),
                "vi_visual" => self.vi_visual.update(val, path, errors),
                "multiline" => self.multiline.update(val, path, errors),
                "transient" => self.transient.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Span, record};

    /// Deliberately uses non-default values, so a getter wired to the wrong
    /// field cannot pass by coincidence.
    fn test_pair() -> (PromptConfig, Value) {
        (
            PromptConfig {
                indicator: "$ ".into(),
                vi_insert: "i ".into(),
                vi_normal: "n ".into(),
                vi_visual: "V ".into(),
                multiline: ".. ".into(),
                transient: TransientPromptConfig {
                    indicator: Some("t$ ".into()),
                    vi_insert: Some("ti ".into()),
                    vi_normal: Some("tn ".into()),
                    // Left null to cover the "keep the live prompt" case.
                    vi_visual: None,
                    multiline: Some("t.. ".into()),
                },
            },
            Value::test_record(record! {
                "indicator" => Value::test_string("$ "),
                "vi_insert" => Value::test_string("i "),
                "vi_normal" => Value::test_string("n "),
                "vi_visual" => Value::test_string("V "),
                "multiline" => Value::test_string(".. "),
                "transient" => Value::test_record(record! {
                    "indicator" => Value::test_string("t$ "),
                    "vi_insert" => Value::test_string("ti "),
                    "vi_normal" => Value::test_string("tn "),
                    "vi_visual" => Value::test_nothing(),
                    "multiline" => Value::test_string("t.. "),
                }),
            }),
        )
    }

    #[test]
    fn update() {
        let (expected, input) = test_pair();
        let config = Config::default();
        let mut errors = ConfigErrors::new(&config);
        let mut result = PromptConfig::default();
        result.update(&input, &mut ConfigPath::new(), &mut errors);
        assert!(!errors.has_errors(), "errors: {errors:#?}");
        assert_eq!(expected, result);
    }

    #[test]
    fn reconstruct() {
        let (input, expected) = test_pair();
        assert_eq!(expected, input.into_value(Span::test_data()));
    }

    #[test]
    fn unknown_key_is_rejected() {
        let config = Config::default();
        let mut errors = ConfigErrors::new(&config);
        let mut result = PromptConfig::default();
        result.update(
            &Value::test_record(record! { "indicatorr" => Value::test_string("> ") }),
            &mut ConfigPath::new(),
            &mut errors,
        );
        assert!(errors.has_errors());
    }

    #[test]
    fn non_string_value_is_rejected() {
        let config = Config::default();
        let mut errors = ConfigErrors::new(&config);
        let mut result = PromptConfig::default();
        result.update(
            &Value::test_record(record! { "indicator" => Value::test_int(1) }),
            &mut ConfigPath::new(),
            &mut errors,
        );
        assert!(errors.has_errors());
    }

    /// Applies `transient` to a default config and hands back the result
    /// alongside whether the update reported an error.
    fn update_transient(transient: Value) -> (TransientPromptConfig, bool) {
        let config = Config::default();
        let mut errors = ConfigErrors::new(&config);
        let mut result = PromptConfig::default();
        result.update(
            &Value::test_record(record! { "transient" => transient }),
            &mut ConfigPath::new(),
            &mut errors,
        );
        (result.transient, errors.has_errors())
    }

    #[test]
    fn transient_multiline_defaults_to_empty_not_null() {
        // Mirrors the TRANSIENT_PROMPT_MULTILINE_INDICATOR that `src/main.rs`
        // used to seed: submitted multiline entries drop the continuation
        // marker so they stay copyable out of scrollback.
        let transient = TransientPromptConfig::default();
        assert_eq!(transient.multiline.as_deref(), Some(""));
        assert_eq!(transient.indicator, None);
    }

    #[test]
    fn transient_null_means_keep_the_live_indicator() {
        // `null` is a value here, not a type error: it clears the default so
        // the live prompt's indicator survives into the transient prompt.
        let (transient, has_errors) = update_transient(Value::test_record(record! {
            "multiline" => Value::test_nothing(),
        }));

        assert!(!has_errors);
        assert_eq!(transient.multiline, None);
    }

    #[test]
    fn transient_accepts_a_string_override() {
        let (transient, has_errors) = update_transient(Value::test_record(record! {
            "indicator" => Value::test_string("t> "),
        }));

        assert!(!has_errors);
        assert_eq!(transient.indicator.as_deref(), Some("t> "));
        // Untouched keys keep their defaults.
        assert_eq!(transient.multiline.as_deref(), Some(""));
    }

    #[test]
    fn transient_rejects_a_non_string_value() {
        let (_, has_errors) = update_transient(Value::test_record(record! {
            "indicator" => Value::test_int(1),
        }));

        assert!(has_errors);
    }

    #[test]
    fn transient_rejects_an_unknown_key() {
        let (_, has_errors) = update_transient(Value::test_record(record! {
            "indicatorr" => Value::test_string("t> "),
        }));

        assert!(has_errors);
    }
}
