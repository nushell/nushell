use std::{cell::RefCell, rc::Rc};

use nu_engine::eval_block;
use nu_parser::{flatten_block, parse};
use nu_protocol::{
    engine::{EngineState, EvaluationContext, Stack, StateWorkingSet},
    Value,
};
use reedline::Completer;

pub struct NuCompleter {
    engine_state: Rc<RefCell<EngineState>>,
}

impl NuCompleter {
    pub fn new(engine_state: Rc<RefCell<EngineState>>) -> Self {
        Self { engine_state }
    }
}

impl Completer for NuCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<(reedline::Span, String)> {
        let engine_state = self.engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);
        let offset = working_set.next_span_start();
        let pos = offset + pos;
        let (output, _err) = parse(&mut working_set, Some("completer"), line.as_bytes(), false);

        let flattened = flatten_block(&working_set, &output);

        for flat in flattened {
            if pos >= flat.0.start && pos <= flat.0.end {
                match &flat.1 {
                    nu_parser::FlatShape::Custom(custom_completion) => {
                        let prefix = working_set.get_span_contents(flat.0).to_vec();

                        let (block, ..) =
                            parse(&mut working_set, None, custom_completion.as_bytes(), false);
                        let context = EvaluationContext {
                            engine_state: self.engine_state.clone(),
                            stack: Stack::default(),
                        };
                        let result = eval_block(&context, &block, Value::nothing());

                        let v: Vec<_> = match result {
                            Ok(Value::List { vals, .. }) => vals
                                .into_iter()
                                .map(move |x| {
                                    let s = x.as_string().expect("FIXME");

                                    (
                                        reedline::Span {
                                            start: flat.0.start - offset,
                                            end: flat.0.end - offset,
                                        },
                                        s,
                                    )
                                })
                                .filter(|x| x.1.as_bytes().starts_with(&prefix))
                                .collect(),
                            _ => vec![],
                        };

                        return v;
                    }
                    nu_parser::FlatShape::External | nu_parser::FlatShape::InternalCall => {
                        let prefix = working_set.get_span_contents(flat.0);
                        let results = working_set.find_commands_by_prefix(prefix);

                        return results
                            .into_iter()
                            .map(move |x| {
                                (
                                    reedline::Span {
                                        start: flat.0.start - offset,
                                        end: flat.0.end - offset,
                                    },
                                    String::from_utf8_lossy(&x).to_string(),
                                )
                            })
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        vec![]
    }
}
