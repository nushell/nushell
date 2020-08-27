use crate::completion::{self, Suggestion};
use crate::context;

pub(crate) struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::Context,
    ) -> (usize, Vec<Suggestion>) {
        use crate::completion::engine::LocationType;

        let nu_context: &context::Context = context.as_ref();
        let lite_block = match nu_parser::lite_parse(line, 0) {
            Ok(block) => Some(block),
            Err(result) => result.partial,
        };

        let locations = lite_block
            .map(|block| nu_parser::classify_block(&block, &nu_context.registry))
            .map(|block| crate::completion::engine::completion_location(line, &block.block, pos))
            .unwrap_or_default();

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let pos = locations[0].span.start();
            let suggestions = locations
                .into_iter()
                .flat_map(|location| {
                    let partial = location.span.slice(line);
                    match location.item {
                        LocationType::Command => {
                            let command_completer = crate::completion::command::Completer {};
                            command_completer.complete(context, partial)
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = crate::completion::flag::Completer {};
                            flag_completer.complete(context, cmd, partial)
                        }

                        LocationType::Argument(_cmd, _arg_name) => {
                            // TODO use cmd and arg_name to narrow things down further
                            let path_completer = crate::completion::path::Completer::new();
                            path_completer.complete(context, partial)
                        }

                        LocationType::Variable => Vec::new(),
                    }
                    .into_iter()
                    .map(requote)
                })
                .collect();

            (pos, suggestions)
        }
    }
}

fn requote(item: Suggestion) -> Suggestion {
    let unescaped = rustyline::completion::unescape(&item.replacement, Some('\\'));
    if unescaped != item.replacement {
        Suggestion {
            display: item.display,
            replacement: format!("\"{}\"", unescaped),
        }
    } else {
        item
    }
}
