use crate::{TextStyle, color_record_to_nustyle, lookup_ansi_color_style, text_style::Alignment};
use nu_ansi_term::Style;
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Span, Value, default_color_config,
    engine::{Closure, EngineState, Stack},
    report_shell_error,
};
use std::{collections::HashMap, fmt::Debug};

// ComputableStyle represents the valid user style types: a single color value, or a closure which
// takes an input value and produces a color value. The latter represents a value which
// is computed at use-time.
#[derive(Debug, Clone)]
pub enum ComputableStyle {
    Static(Style),
    Closure(Closure, Span),
}

// An alias for the mapping used internally by StyleComputer.
pub type StyleMapping = HashMap<String, ComputableStyle>;

// A StyleComputer is an all-in-one way to compute styles. A nu command can
// simply create it with from_config(), and then use it with compute().
// It stores the engine state and stack needed to run closures that
// may be defined as a user style.

#[derive(Debug)]
pub struct StyleComputer<'a> {
    engine_state: &'a EngineState,
    stack: &'a Stack,
    map: StyleMapping,
}

/// Insert a color_config value into a style map (string / record / closure).
fn insert_style_from_value(map: &mut StyleMapping, key: String, value: &Value) {
    let span = value.span();
    match value {
        Value::Closure { val, .. } => {
            map.insert(key, ComputableStyle::Closure(*val.clone(), span));
        }
        Value::Record { .. } => {
            map.insert(key, ComputableStyle::Static(color_record_to_nustyle(value)));
        }
        Value::String { val, .. } => {
            map.insert(
                key,
                ComputableStyle::Static(lookup_ansi_color_style(val.as_str())),
            );
        }
        // Unsupported types are ignored (same as pre-existing from_config behavior).
        _ => (),
    }
}

impl<'a> StyleComputer<'a> {
    // This is NOT meant to be used in most cases - please use from_config() instead.
    // This only exists for testing purposes.
    pub fn new(
        engine_state: &'a EngineState,
        stack: &'a Stack,
        map: StyleMapping,
    ) -> StyleComputer<'a> {
        StyleComputer {
            engine_state,
            stack,
            map,
        }
    }
    // The main method. Takes a string name which maps to a color_config style name,
    // and a Nu value to pipe into any closures that may have been defined there.
    pub fn compute(&self, style_name: &str, value: &Value) -> Style {
        match self.map.get(style_name) {
            // Static values require no computation.
            Some(ComputableStyle::Static(s)) => *s,
            // Closures are run here.
            Some(ComputableStyle::Closure(closure, span)) => {
                let result = ClosureEvalOnce::new(self.engine_state, self.stack, closure.clone())
                    .debug(false)
                    .run_with_value(value.clone())
                    .and_then(|data| data.into_value(*span));

                match result {
                    Ok(value) => {
                        // These should be the same color data forms supported by color_config.
                        match value {
                            Value::Record { .. } => color_record_to_nustyle(&value),
                            Value::String { val, .. } => lookup_ansi_color_style(&val),
                            _ => Style::default(),
                        }
                    }
                    Err(err) => {
                        report_shell_error(Some(self.stack), self.engine_state, &err);
                        Style::default()
                    }
                }
            }
            // There should be no other kinds of values (due to create_map() in config.rs filtering them out)
            // so this is just a fallback.
            _ => Style::default(),
        }
    }

    // Used only by the `table` command.
    pub fn style_primitive(&self, value: &Value) -> TextStyle {
        use Alignment::*;
        let s = self.compute(&value.get_type_shallow().get_non_specified_string(), value);
        match *value {
            Value::Bool { .. } => TextStyle::with_style(Left, s),
            Value::Int { .. } => TextStyle::with_style(Right, s),
            Value::Filesize { .. } => TextStyle::with_style(Right, s),
            Value::Duration { .. } => TextStyle::with_style(Right, s),
            Value::Date { .. } => TextStyle::with_style(Left, s),
            Value::Range { .. } => TextStyle::with_style(Left, s),
            Value::Float { .. } => TextStyle::with_style(Right, s),
            Value::String { .. } => TextStyle::with_style(Left, s),
            Value::Glob { .. } => TextStyle::with_style(Left, s),
            Value::Nothing { .. } => TextStyle::with_style(Left, s),
            Value::Binary { .. } => TextStyle::with_style(Left, s),
            Value::CellPath { .. } => TextStyle::with_style(Left, s),
            Value::Record { .. } | Value::List { .. } => TextStyle::with_style(Left, s),
            Value::Closure { .. } | Value::Error { .. } => TextStyle::basic_left(),
            Value::Custom { ref val, .. } => {
                let type_name = val.type_name();
                let custom_style = self.compute(&type_name, value);
                TextStyle::with_style(Left, custom_style)
            }
        }
    }

    // The main constructor.
    //
    // Defaults come from `default_color_config()` (same source as
    // `Config::default().color_config`). We seed with those defaults, then overlay
    // the live `color_config` so partial theme assignments that omit keys
    // (e.g. `binary_*`) still fall back to the built-in styles instead of plain
    // `Style::default()`.
    pub fn from_config(engine_state: &'a EngineState, stack: &'a Stack) -> StyleComputer<'a> {
        let config = stack.get_config(engine_state);

        let mut map: StyleMapping = HashMap::new();

        for (key, value) in default_color_config() {
            insert_style_from_value(&mut map, key, &value);
        }

        for (key, value) in &config.color_config {
            insert_style_from_value(&mut map, key.clone(), value);
        }

        StyleComputer::new(engine_state, stack, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Config;
    use std::sync::Arc;

    #[test]
    fn test_computable_style_static() {
        let style1 = Style::default().italic();
        let style2 = Style::default().underline();
        // Create a "dummy" style_computer for this test.
        let dummy_engine_state = EngineState::new();
        let dummy_stack = Stack::new();
        let style_computer = StyleComputer::new(
            &dummy_engine_state,
            &dummy_stack,
            [
                ("string".into(), ComputableStyle::Static(style1)),
                ("row_index".into(), ComputableStyle::Static(style2)),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            style_computer.compute("string", &Value::nothing(Span::unknown())),
            style1
        );
        assert_eq!(
            style_computer.compute("row_index", &Value::nothing(Span::unknown())),
            style2
        );
    }

    #[test]
    fn partial_color_config_falls_back_to_defaults_for_binary() {
        // Simulates `$env.config.color_config = { header: red }` which replaces
        // the whole map and drops keys the theme never set (e.g. binary_*).
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        let config = Config {
            color_config: [("header".to_string(), Value::test_string("red"))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        stack.config = Some(Arc::new(config));

        let style_computer = StyleComputer::from_config(&engine_state, &stack);
        let null = Value::nothing(Span::unknown());

        assert_eq!(
            style_computer.compute("header", &null),
            lookup_ansi_color_style("red")
        );
        assert_eq!(
            style_computer.compute("binary_null_char", &null),
            lookup_ansi_color_style("grey42")
        );
        assert_eq!(
            style_computer.compute("binary_printable", &null),
            lookup_ansi_color_style("cyan_bold")
        );
        assert_eq!(
            style_computer.compute("binary_whitespace", &null),
            lookup_ansi_color_style("green_bold")
        );
        assert_eq!(
            style_computer.compute("binary_ascii_other", &null),
            lookup_ansi_color_style("purple_bold")
        );
        assert_eq!(
            style_computer.compute("binary_non_ascii", &null),
            lookup_ansi_color_style("yellow_bold")
        );
    }
}
