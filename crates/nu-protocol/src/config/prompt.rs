use super::prelude::*;
use crate as nu_protocol;

/// Everything the interactive prompt is built from.
///
/// Each key replaces one `$env.PROMPT_*` variable, which still takes
/// precedence when set but is deprecated.
#[derive(Clone, Debug, IntoValue, PartialEq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// The main prompt: a string used verbatim, or a closure evaluated once
    /// per cycle. `None` falls back to the line editor's built-in prompt.
    pub left: Option<Value>,
    /// The right-aligned prompt, same shape as [`PromptConfig::left`].
    pub right: Option<Value>,
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
    /// Whether the right prompt sits on the last line of a multi-line left
    /// prompt. Renamed from `$env.config.render_right_prompt_on_last_line`.
    pub render_right_on_last_line: bool,
    /// Overrides applied once a line is submitted and the prompt is redrawn.
    pub transient: TransientPromptConfig,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            // A closure needs a parsed block, so these cannot default here.
            // `default_env.nu` fills them in.
            left: None,
            right: None,
            indicator: "> ".into(),
            vi_insert: ": ".into(),
            vi_normal: "> ".into(),
            vi_visual: "v ".into(),
            multiline: "::: ".into(),
            render_right_on_last_line: false,
            transient: TransientPromptConfig::default(),
        }
    }
}

/// The transient prompt, which replaces the real one in scrollback once a line
/// is submitted.
///
/// A `null` field keeps whatever the live prompt showed, so only the segments
/// worth condensing need spelling out. Each key replaces one
/// `$env.TRANSIENT_PROMPT_*` variable, deprecated but still winning when set.
#[derive(Clone, Debug, IntoValue, PartialEq, Serialize, Deserialize)]
pub struct TransientPromptConfig {
    /// Replaces [`PromptConfig::left`], with the same string-or-closure shape.
    pub left: Option<Value>,
    /// Replaces [`PromptConfig::right`]. Defaults to empty rather than `null`,
    /// matching the seed `src/main.rs` used to install: a right prompt is
    /// usually a timestamp, which is noise once the line is submitted.
    pub right: Option<Value>,
    /// Replaces [`PromptConfig::indicator`].
    pub indicator: Option<String>,
    /// Replaces [`PromptConfig::vi_insert`].
    pub vi_insert: Option<String>,
    /// Replaces [`PromptConfig::vi_normal`].
    pub vi_normal: Option<String>,
    /// Replaces [`PromptConfig::vi_visual`].
    pub vi_visual: Option<String>,
    /// Replaces [`PromptConfig::multiline`]. Defaults to empty rather than
    /// `null`: dropping the continuation marker keeps submitted multiline
    /// entries copyable out of scrollback.
    pub multiline: Option<String>,
}

impl Default for TransientPromptConfig {
    fn default() -> Self {
        Self {
            left: None,
            right: Some(Value::string(String::new(), Span::unknown())),
            indicator: None,
            vi_insert: None,
            vi_normal: None,
            vi_visual: None,
            multiline: Some(String::new()),
        }
    }
}

/// A string, a closure, or `null` to clear the key. Shared by the live and
/// transient keys so both stay interchangeable with the variables they
/// replace.
fn update_prompt_source<'a>(
    field: &mut Option<Value>,
    value: &'a Value,
    path: &mut ConfigPath<'a>,
    errors: &mut ConfigErrors,
) {
    match value {
        Value::Nothing { .. } => *field = None,
        Value::String { .. } | Value::Closure { .. } => *field = Some(value.clone()),
        _ => errors.type_mismatch(path, Type::custom("string, closure or nothing"), value),
    }
}

/// A string, or `null` to keep whatever the live prompt showed. `null` being a
/// value rather than a type error is what separates these from the live
/// prompt's indicators.
fn update_optional_string<'a>(
    field: &mut Option<String>,
    value: &'a Value,
    path: &mut ConfigPath<'a>,
    errors: &mut ConfigErrors,
) {
    match value {
        Value::Nothing { .. } => *field = None,
        Value::String { val, .. } => *field = Some(val.clone()),
        _ => errors.type_mismatch(path, Type::custom("string or nothing"), value),
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
            match col.as_str() {
                "left" => update_prompt_source(&mut self.left, val, path, errors),
                "right" => update_prompt_source(&mut self.right, val, path, errors),
                "indicator" => update_optional_string(&mut self.indicator, val, path, errors),
                "vi_insert" => update_optional_string(&mut self.vi_insert, val, path, errors),
                "vi_normal" => update_optional_string(&mut self.vi_normal, val, path, errors),
                "vi_visual" => update_optional_string(&mut self.vi_visual, val, path, errors),
                "multiline" => update_optional_string(&mut self.multiline, val, path, errors),
                _ => errors.unknown_option(path, val),
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
                "left" => update_prompt_source(&mut self.left, val, path, errors),
                "right" => update_prompt_source(&mut self.right, val, path, errors),
                "indicator" => self.indicator.update(val, path, errors),
                "vi_insert" => self.vi_insert.update(val, path, errors),
                "vi_normal" => self.vi_normal.update(val, path, errors),
                "vi_visual" => self.vi_visual.update(val, path, errors),
                "multiline" => self.multiline.update(val, path, errors),
                "render_right_on_last_line" => {
                    self.render_right_on_last_line.update(val, path, errors)
                }
                "transient" => self.transient.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockId, Config, Span, engine::Closure, record};

    /// Deliberately uses non-default values, so a getter wired to the wrong
    /// field cannot pass by coincidence.
    fn test_pair() -> (PromptConfig, Value) {
        (
            PromptConfig {
                left: Some(Value::test_string("left ")),
                right: Some(Value::test_string("right ")),
                indicator: "$ ".into(),
                vi_insert: "i ".into(),
                vi_normal: "n ".into(),
                vi_visual: "V ".into(),
                multiline: ".. ".into(),
                render_right_on_last_line: true,
                transient: TransientPromptConfig {
                    left: Some(Value::test_string("tleft ")),
                    right: Some(Value::test_string("tright ")),
                    indicator: Some("t$ ".into()),
                    vi_insert: Some("ti ".into()),
                    vi_normal: Some("tn ".into()),
                    // Left null to cover the "keep the live prompt" case.
                    vi_visual: None,
                    multiline: Some("t.. ".into()),
                },
            },
            Value::test_record(record! {
                "left" => Value::test_string("left "),
                "right" => Value::test_string("right "),
                "indicator" => Value::test_string("$ "),
                "vi_insert" => Value::test_string("i "),
                "vi_normal" => Value::test_string("n "),
                "vi_visual" => Value::test_string("V "),
                "multiline" => Value::test_string(".. "),
                "render_right_on_last_line" => Value::test_bool(true),
                "transient" => Value::test_record(record! {
                    "left" => Value::test_string("tleft "),
                    "right" => Value::test_string("tright "),
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

    /// A closure `Value`. Config never runs the block, so a dummy id is enough
    /// to cover the "a closure is a valid prompt source" path.
    fn test_closure() -> Value {
        Value::closure(
            Closure {
                block_id: BlockId::new(0),
                captures: Vec::new(),
            },
            Span::test_data(),
        )
    }

    /// Applies `record` to a default config and hands back the result
    /// alongside whether the update reported an error.
    fn update_prompt(record: Value) -> (PromptConfig, bool) {
        let config = Config::default();
        let mut errors = ConfigErrors::new(&config);
        let mut result = PromptConfig::default();
        result.update(&record, &mut ConfigPath::new(), &mut errors);
        (result, errors.has_errors())
    }

    #[test]
    fn left_accepts_a_string_or_a_closure() {
        // Both shapes have to survive the round trip: `nu-cli` uses a string
        // verbatim and evaluates a closure, and the variables being replaced
        // always took either.
        for source in [Value::test_string("> "), test_closure()] {
            let (prompt, has_errors) =
                update_prompt(Value::test_record(record! { "left" => source.clone() }));

            assert!(!has_errors);
            assert_eq!(prompt.left, Some(source));
        }
    }

    #[test]
    fn left_null_means_the_line_editor_default() {
        let (prompt, has_errors) = update_prompt(Value::test_record(record! {
            "left" => Value::test_nothing(),
        }));

        assert!(!has_errors);
        assert_eq!(prompt.left, None);
    }

    #[test]
    fn left_rejects_a_value_that_cannot_render() {
        let (_, has_errors) = update_prompt(Value::test_record(record! {
            "left" => Value::test_int(1),
        }));

        assert!(has_errors);
    }

    #[test]
    fn transient_right_defaults_to_empty_not_null() {
        // Mirrors the TRANSIENT_PROMPT_COMMAND_RIGHT that `src/main.rs` used to
        // seed: a right prompt is usually a timestamp, which is noise once the
        // line has been submitted.
        let transient = TransientPromptConfig::default();
        assert_eq!(
            transient.right.as_ref().and_then(|val| val.as_str().ok()),
            Some("")
        );
        assert_eq!(transient.left, None);
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
