use crate::{TextStyle, color_record_to_nustyle, lookup_ansi_color_style, text_style::Alignment};
use nu_ansi_term::{Color, Style};
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Span, Value,
    engine::{Closure, EngineState, Stack},
    report_shell_error,
};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter, Result},
};

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
//
// A StyleComputer is an all-in-one way to compute styles. A nu command can
// simply create it with from_config(), and then use it with compute().
// It stores the engine state and stack needed to run closures that
// may be defined as a user style.
//
pub struct StyleComputer<'a> {
    engine_state: &'a EngineState,
    stack: &'a Stack,
    map: StyleMapping,
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
                        report_shell_error(self.engine_state, &err);
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
            Value::Glob { .. } => TextStyle::with_style(Left, s),
            Value::Nothing { .. } => TextStyle::with_style(Left, s),
            Value::Binary { .. } => TextStyle::with_style(Left, s),
            Value::CellPath { .. } => TextStyle::with_style(Left, s),
            Value::Record { .. } | Value::List { .. } => TextStyle::with_style(Left, s),
            Value::Closure { .. } | Value::Custom { .. } | Value::Error { .. } => {
                TextStyle::basic_left()
            }
        }
    }

    // The main constructor.
    pub fn from_config(engine_state: &'a EngineState, stack: &'a Stack) -> StyleComputer<'a> {
        let config = stack.get_config(engine_state);

        // Create the hashmap
        #[rustfmt::skip]
        let mut map: StyleMapping = [
            ("separator".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("leading_trailing_space_bg".to_string(), ComputableStyle::Static(Style::default().on(Color::Rgb(128, 128, 128)))),
            ("header".to_string(), ComputableStyle::Static(Color::Green.bold())),
            ("empty".to_string(), ComputableStyle::Static(Color::Blue.normal())),
            ("bool".to_string(), ComputableStyle::Static(Color::LightCyan.normal())),
            ("int".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("filesize".to_string(), ComputableStyle::Static(Color::Cyan.normal())),
            ("duration".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("datetime".to_string(), ComputableStyle::Static(Color::Purple.normal())),
            ("range".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("float".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("string".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("nothing".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("binary".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("cell-path".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("row_index".to_string(), ComputableStyle::Static(Color::Green.bold())),
            ("record".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("list".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("block".to_string(), ComputableStyle::Static(Color::Default.normal())),
            ("hints".to_string(), ComputableStyle::Static(Color::DarkGray.normal())),
            ("search_result".to_string(), ComputableStyle::Static(Color::Default.normal().on(Color::Red))),
        ].into_iter().collect();

        for (key, value) in &config.color_config {
            let span = value.span();
            match value {
                Value::Closure { val, .. } => {
                    map.insert(
                        key.to_string(),
                        ComputableStyle::Closure(*val.clone(), span),
                    );
                }
                Value::Record { .. } => {
                    map.insert(
                        key.to_string(),
                        ComputableStyle::Static(color_record_to_nustyle(value)),
                    );
                }
                Value::String { val, .. } => {
                    // update the stylemap with the found key
                    let color = lookup_ansi_color_style(val.as_str());
                    if let Some(v) = map.get_mut(key) {
                        *v = ComputableStyle::Static(color);
                    } else {
                        map.insert(key.to_string(), ComputableStyle::Static(color));
                    }
                }
                // This should never occur.
                _ => (),
            }
        }
        StyleComputer::new(engine_state, stack, map)
    }
}

// Because EngineState doesn't have Debug (Dec 2022),
// this incomplete representation must be used.
impl Debug for StyleComputer<'_> {
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
        assert_eq!(actual_repl.out, r#"["bell.obj", "book.obj", "candle.obj"]"#);
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
    assert!(
        actual_repl
            .err
            .contains("nu::shell::operator_incompatible_types")
    );
    // Check that the value was printed
    assert!(actual_repl.out.contains("bell"));
}
