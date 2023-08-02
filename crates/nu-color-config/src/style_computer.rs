use crate::text_style::Alignment;
use crate::{color_record_to_nustyle, lookup_ansi_color_style, TextStyle};
use nu_ansi_term::{Color, Style};
use nu_engine::env::get_config;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use std::collections::HashMap;

use std::fmt::{Debug, Formatter, Result};

// An alias for the mapping used internally by StyleComputer.
pub type StyleMapping = HashMap<String, Style>;
//
// A StyleComputer is an all-in-one way to compute styles. A nu command can
// simply create it with from_config(), and then use it with compute().
// It stores the engine state and stack needed to run closures that
// may be defined as a user style.
//
pub struct StyleComputer {
    map: StyleMapping,
}

impl<'a> StyleComputer {
    // This is NOT meant to be used in most cases - please use from_config() instead.
    // This only exists for testing purposes.
    pub fn new(map: StyleMapping) -> StyleComputer {
        StyleComputer { map }
    }
    // The main method. Takes a string name which maps to a color_config style name,
    // and a Nu value to pipe into any closures that may have been defined there.
    pub fn compute(&self, style_name: &str, _value: &Value) -> Style {
        match self.map.get(style_name) {
            // Static values require no computation.
            Some(s) => *s,
            // Closures are run here.
            // There should be no other kinds of values (due to create_map() in config.rs filtering them out)
            // so this is just a fallback.
            _ => Style::default(),
        }
    }

    // Used only by the `table` command.
    pub fn style_primitive(&self, value: &Value) -> TextStyle {
        use Alignment::*;
        let s = self.compute(&value.get_type().get_non_specified_string(), value);
        match *value {
            Value::Bool { .. } => TextStyle::with_style(Left, s),
            Value::Int { .. } => TextStyle::with_style(Right, s),
            Value::Filesize { .. } => TextStyle::with_style(Right, s),
            Value::Duration { .. } => TextStyle::with_style(Right, s),
            Value::Date { .. } => TextStyle::with_style(Left, s),
            Value::Range { .. } => TextStyle::with_style(Left, s),
            Value::Float { .. } => TextStyle::with_style(Right, s),
            Value::String { .. } => TextStyle::with_style(Left, s),
            Value::Nothing { .. } => TextStyle::with_style(Left, s),
            Value::Binary { .. } => TextStyle::with_style(Left, s),
            Value::CellPath { .. } => TextStyle::with_style(Left, s),
            Value::Record { .. } | Value::List { .. } | Value::Block { .. } => {
                TextStyle::with_style(Left, s)
            }
            Value::Closure { .. }
            | Value::CustomValue { .. }
            | Value::Error { .. }
            | Value::LazyRecord { .. }
            | Value::MatchPattern { .. } => TextStyle::basic_left(),
        }
    }

    // The main constructor.
    pub fn from_config(engine_state: &'a EngineState, stack: &'a Stack) -> StyleComputer {
        let config = get_config(engine_state, stack);

        // Create the hashmap
        #[rustfmt::skip]
        let mut map: StyleMapping = [
            ("separator".to_string(), Color::White.normal()),
            ("leading_trailing_space_bg".to_string(), Style::new()),
            ("header".to_string(), Color::Green.bold()),
            ("empty".to_string(), Color::Blue.normal()),
            // FIXME: add value-specific colors?
            ("bool".to_string(), Color::LightCyan.normal()),
            ("int".to_string(), Color::White.normal()),
            // FIXME: add value-specific colors?
            ("filesize".to_string(), Color::Cyan.normal()),
            ("duration".to_string(), Color::White.normal()),
            // FIXME: add value-specific colors?
            ("date".to_string(), Color::Purple.normal()),
            ("range".to_string(), Color::White.normal()),
            ("float".to_string(), Color::White.normal()),
            ("string".to_string(), Color::White.normal()),
            ("nothing".to_string(), Color::White.normal()),
            ("binary".to_string(), Color::White.normal()),
            ("cellpath".to_string(), Color::White.normal()),
            ("row_index".to_string(), Color::Green.bold()),
            ("record".to_string(), Color::White.normal()),
            ("list".to_string(), Color::White.normal()),
            ("block".to_string(), Color::White.normal()),
            ("hints".to_string(), Color::DarkGray.normal()),
            ("search_result".to_string(), {
                let mut style = Style::new().fg(Color::White);
                style.background = Some(Color::Red);
                style
            }),
        ].into_iter().collect();

        for (key, value) in &config.color_config {
            match value {
                Value::Record { .. } => {
                    map.insert(key.to_string(), color_record_to_nustyle(value));
                }
                Value::String { val, .. } => {
                    // update the stylemap with the found key
                    let color = lookup_ansi_color_style(val.as_str());
                    if let Some(v) = map.get_mut(key) {
                        *v = color;
                    } else {
                        map.insert(key.to_string(), color);
                    }
                }
                // This should never occur.
                _ => (),
            }
        }
        StyleComputer::new(map)
    }
}

// Because EngineState doesn't have Debug (Dec 2022),
// this incomplete representation must be used.
impl Debug for StyleComputer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("StyleComputer")
            .field("map", &self.map)
            .finish()
    }
}

#[test]
fn test_computable_style_static() {
    use nu_protocol::Span;

    let style1 = Style::default().italic();
    let style2 = Style::default().underline();
    let style_computer = StyleComputer::new(
        [("string".into(), style1), ("row_index".into(), style2)]
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

// Because each closure currently runs in a separate environment, checks that the closures have run
// must use the filesystem.
#[test]
fn test_computable_style_closure_basic() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("computable_style_closure_basic", |dirs, _| {
        let inp = [
            r#"$env.config = {
                color_config: {
                    string: {|e| touch ($e + '.obj'); 'red' }
                }
            };"#,
            "[bell book candle] | table | ignore",
            "ls | get name | to nuon",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "[bell.obj, book.obj, candle.obj]");
    });
}

#[test]
fn test_computable_style_closure_errors() {
    use nu_test_support::{nu, nu_repl_code};
    let inp = [
        r#"$env.config = {
            color_config: {
                string: {|e| $e + 2 }
            }
        };"#,
        "[bell] | table",
    ];
    let actual_repl = nu!(nu_repl_code(&inp));
    // Check that the error was printed
    assert!(actual_repl.err.contains("type mismatch for operator"));
    // Check that the value was printed
    assert!(actual_repl.out.contains("bell"));
}
