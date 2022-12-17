use crate::{color_record_to_nustyle, lookup_ansi_color_style, TextStyle};
use nu_ansi_term::{Color, Style};
use nu_engine::eval_block;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    CliError, IntoPipelineData, Value,
};
use tabled::alignment::AlignmentHorizontal;

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
    Closure(Value),
}

// macro used for adding initial values to the style hashmap
macro_rules! initial {
    ($a:expr, $b:expr) => {
        ($a.to_string(), ComputableStyle::Static($b))
    };
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
            Some(ComputableStyle::Closure(Value::Closure {
                val: block_id,
                captures,
                span,
            })) => {
                let block = self.engine_state.get_block(*block_id).clone();
                // Because captures_to_stack() clones, we don't need to use with_env() here
                // (contrast with_env() usage in `each` or `do`).
                let mut stack = self.stack.captures_to_stack(captures);

                // Support 1-argument blocks as well as 0-argument blocks.
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, value.clone());
                    }
                }

                // Run the block.
                match eval_block(
                    self.engine_state,
                    &mut stack,
                    &block,
                    value.clone().into_pipeline_data(),
                    false,
                    false,
                ) {
                    Ok(v) => {
                        let value = v.into_value(*span);
                        // These should be the same color data forms supported by color_config.
                        match value {
                            Value::Record { .. } => color_record_to_nustyle(&value),
                            Value::String { val, .. } => lookup_ansi_color_style(&val),
                            _ => Style::default(),
                        }
                    }
                    // This is basically a copy of nu_cli::report_error(), but that isn't usable due to
                    // dependencies. While crudely spitting out a bunch of errors like this is not ideal,
                    // currently hook closure errors behave roughly the same.
                    Err(e) => {
                        eprintln!(
                            "Error: {:?}",
                            CliError(&e, &StateWorkingSet::new(self.engine_state))
                        );
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
        let s = self.compute(&value.get_type().to_string(), value);
        match *value {
            Value::Bool { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Int { .. } => TextStyle::with_style(AlignmentHorizontal::Right, s),

            Value::Filesize { .. } => TextStyle::with_style(AlignmentHorizontal::Right, s),

            Value::Duration { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Date { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Range { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Float { .. } => TextStyle::with_style(AlignmentHorizontal::Right, s),

            Value::String { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Nothing { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Binary { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::CellPath { .. } => TextStyle::with_style(AlignmentHorizontal::Left, s),

            Value::Record { .. } | Value::List { .. } | Value::Block { .. } => {
                TextStyle::with_style(AlignmentHorizontal::Left, s)
            }
            _ => TextStyle::basic_left(),
        }
    }

    // The main constructor.
    pub fn from_config(engine_state: &'a EngineState, stack: &'a Stack) -> StyleComputer<'a> {
        let config = engine_state.get_config();

        // Create the hashmap
        let mut map: StyleMapping = HashMap::from([
            initial!("separator", Color::White.normal()),
            initial!(
                "leading_trailing_space_bg",
                Style::default().on(Color::Rgb(128, 128, 128))
            ),
            initial!("header", Color::White.normal()),
            initial!("empty", Color::White.normal()),
            initial!("bool", Color::White.normal()),
            initial!("int", Color::White.normal()),
            initial!("filesize", Color::White.normal()),
            initial!("duration", Color::White.normal()),
            initial!("date", Color::White.normal()),
            initial!("range", Color::White.normal()),
            initial!("float", Color::White.normal()),
            initial!("string", Color::White.normal()),
            initial!("nothing", Color::White.normal()),
            initial!("binary", Color::White.normal()),
            initial!("cellpath", Color::White.normal()),
            initial!("row_index", Color::Green.bold()),
            initial!("record", Color::White.normal()),
            initial!("list", Color::White.normal()),
            initial!("block", Color::White.normal()),
            initial!("hints", Color::DarkGray.normal()),
        ]);

        for (key, value) in &config.color_config {
            match value {
                Value::Closure { .. } => {
                    map.insert(key.to_string(), ComputableStyle::Closure(value.clone()));
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
impl<'a> Debug for StyleComputer<'a> {
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
    let mut dummy_stack = Stack::new();
    let style_computer = StyleComputer::new(
        &dummy_engine_state,
        &mut dummy_stack,
        HashMap::from([
            ("string".into(), ComputableStyle::Static(style1)),
            ("row_index".into(), ComputableStyle::Static(style2)),
        ]),
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
            r#"let-env config = {
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
        r#"let-env config = {
            color_config: {
                string: {|e| $e + 2 }
            }
        };"#,
        "[bell] | table",
    ];
    let actual_repl = nu!(cwd: ".", nu_repl_code(&inp));
    // Check that the error was printed
    assert!(actual_repl.err.contains("type mismatch for operator"));
    // Check that the value was printed
    assert!(actual_repl.out.contains("bell"));
}
