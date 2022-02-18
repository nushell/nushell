use nu_engine::eval_block;
use nu_parser::{flatten_expression, parse, trim_quotes, FlatShape};
use nu_protocol::{
    ast::Expr,
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Value, CONFIG_VARIABLE_ID,
};
use reedline::Completer;

const SEP: char = std::path::MAIN_SEPARATOR;

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: EngineState,
    config: Option<Value>,
}

impl NuCompleter {
    pub fn new(engine_state: EngineState, config: Option<Value>) -> Self {
        Self {
            engine_state,
            config,
        }
    }

    fn external_command_completion(&self, prefix: &str) -> Vec<String> {
        let mut executables = vec![];

        let paths;
        paths = self.engine_state.env_vars.get("PATH");

        if let Some(paths) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.as_string().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path) {
                        while let Some(Ok(item)) = contents.next() {
                            if !executables.contains(
                                &item
                                    .path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            ) && matches!(
                                item.path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().starts_with(prefix)),
                                Some(true)
                            ) && is_executable::is_executable(&item.path())
                            {
                                if let Ok(name) = item.file_name().into_string() {
                                    executables.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        executables
    }

    fn complete_variables(
        &self,
        working_set: &StateWorkingSet,
        prefix: &[u8],
        span: Span,
        offset: usize,
    ) -> Vec<(reedline::Span, String)> {
        let mut output = vec![];

        let builtins = [
            "$nu", "$scope", "$in", "$config", "$env", "$true", "$false", "$nothing",
        ];

        for builtin in builtins {
            if builtin.as_bytes().starts_with(prefix) {
                output.push((
                    reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    builtin.to_string(),
                ));
            }
        }

        for scope in &working_set.delta.scope {
            for v in &scope.vars {
                if v.0.starts_with(prefix) {
                    output.push((
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        String::from_utf8_lossy(v.0).to_string(),
                    ));
                }
            }
        }
        for scope in &self.engine_state.scope {
            for v in &scope.vars {
                if v.0.starts_with(prefix) {
                    output.push((
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        String::from_utf8_lossy(v.0).to_string(),
                    ));
                }
            }
        }

        output.dedup();

        output
    }

    fn complete_commands(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        find_externals: bool,
    ) -> Vec<(reedline::Span, String)> {
        let prefix = working_set.get_span_contents(span);

        let mut results = working_set
            .find_commands_by_prefix(prefix)
            .into_iter()
            .map(move |x| {
                (
                    reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    String::from_utf8_lossy(&x).to_string(),
                )
            })
            .collect::<Vec<_>>();

        let prefix = working_set.get_span_contents(span);
        let prefix = String::from_utf8_lossy(prefix).to_string();
        if find_externals {
            let results_external =
                self.external_command_completion(&prefix)
                    .into_iter()
                    .map(move |x| {
                        (
                            reedline::Span {
                                start: span.start - offset,
                                end: span.end - offset,
                            },
                            x,
                        )
                    });

            for external in results_external {
                if results.contains(&external) {
                    results.push((external.0, format!("^{}", external.1)))
                } else {
                    results.push(external)
                }
            }

            results
        } else {
            results
        }
    }

    fn completion_helper(&self, line: &str, pos: usize) -> Vec<(reedline::Span, String)> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let offset = working_set.next_span_start();
        let mut line = line.to_string();
        line.insert(pos, 'a');
        let pos = offset + pos;
        let (output, _err) = parse(&mut working_set, Some("completer"), line.as_bytes(), false);

        for pipeline in output.pipelines.into_iter() {
            for expr in pipeline.expressions {
                let flattened: Vec<_> = flatten_expression(&working_set, &expr);

                for (flat_idx, flat) in flattened.iter().enumerate() {
                    if pos >= flat.0.start && pos < flat.0.end {
                        let new_span = Span {
                            start: flat.0.start,
                            end: flat.0.end - 1,
                        };

                        let mut prefix = working_set.get_span_contents(flat.0).to_vec();
                        prefix.remove(pos - flat.0.start);

                        if prefix.starts_with(b"$") {
                            return self.complete_variables(
                                &working_set,
                                &prefix,
                                new_span,
                                offset,
                            );
                        }
                        if prefix.starts_with(b"-") {
                            // this might be a flag, let's see
                            if let Expr::Call(call) = &expr.expr {
                                let decl = working_set.get_decl(call.decl_id);
                                let sig = decl.signature();

                                let mut output = vec![];

                                for named in &sig.named {
                                    let mut named = named.long.as_bytes().to_vec();
                                    named.insert(0, b'-');
                                    named.insert(0, b'-');
                                    if named.starts_with(&prefix) {
                                        output.push((
                                            reedline::Span {
                                                start: new_span.start - offset,
                                                end: new_span.end - offset,
                                            },
                                            String::from_utf8_lossy(&named).to_string(),
                                        ));
                                    }
                                }
                                return output;
                            }
                        }

                        match &flat.1 {
                            FlatShape::Custom(custom_completion) => {
                                //let prefix = working_set.get_span_contents(flat.0).to_vec();

                                let (block, ..) = parse(
                                    &mut working_set,
                                    None,
                                    custom_completion.as_bytes(),
                                    false,
                                );

                                let mut stack = Stack::new();
                                // Set up our initial config to start from
                                if let Some(conf) = &self.config {
                                    stack.vars.insert(CONFIG_VARIABLE_ID, conf.clone());
                                } else {
                                    stack.vars.insert(
                                        CONFIG_VARIABLE_ID,
                                        Value::Record {
                                            cols: vec![],
                                            vals: vec![],
                                            span: Span { start: 0, end: 0 },
                                        },
                                    );
                                }

                                let result = eval_block(
                                    &self.engine_state,
                                    &mut stack,
                                    &block,
                                    PipelineData::new(new_span),
                                );

                                let v: Vec<_> = match result {
                                    Ok(pd) => pd
                                        .into_iter()
                                        .filter_map(move |x| {
                                            let s = x.as_string();

                                            match s {
                                                Ok(s) => Some((
                                                    reedline::Span {
                                                        start: new_span.start - offset,
                                                        end: new_span.end - offset,
                                                    },
                                                    s,
                                                )),
                                                Err(_) => None,
                                            }
                                        })
                                        .filter(|x| x.1.as_bytes().starts_with(&prefix))
                                        .collect(),
                                    _ => vec![],
                                };

                                return v;
                            }
                            FlatShape::Filepath | FlatShape::GlobPattern => {
                                let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
                                    match d.as_string() {
                                        Ok(s) => s,
                                        Err(_) => "".to_string(),
                                    }
                                } else {
                                    "".to_string()
                                };
                                let prefix = String::from_utf8_lossy(&prefix).to_string();
                                return file_path_completion(new_span, &prefix, &cwd)
                                    .into_iter()
                                    .map(move |x| {
                                        (
                                            reedline::Span {
                                                start: x.0.start - offset,
                                                end: x.0.end - offset,
                                            },
                                            x.1,
                                        )
                                    })
                                    .collect();
                            }
                            flat_shape => {
                                let last = flattened
                                    .iter()
                                    .rev()
                                    .skip_while(|x| x.0.end > pos)
                                    .take_while(|x| {
                                        matches!(
                                            x.1,
                                            FlatShape::InternalCall
                                                | FlatShape::External
                                                | FlatShape::ExternalArg
                                                | FlatShape::Literal
                                                | FlatShape::String
                                        )
                                    })
                                    .last();

                                // The last item here would be the earliest shape that could possible by part of this subcommand
                                let subcommands = if let Some(last) = last {
                                    self.complete_commands(
                                        &working_set,
                                        Span {
                                            start: last.0.start,
                                            end: pos,
                                        },
                                        offset,
                                        false,
                                    )
                                } else {
                                    vec![]
                                };

                                if !subcommands.is_empty() {
                                    return subcommands;
                                }

                                let commands =
                                    if matches!(flat_shape, nu_parser::FlatShape::External)
                                        || matches!(flat_shape, nu_parser::FlatShape::InternalCall)
                                        || ((new_span.end - new_span.start) == 0)
                                    {
                                        // we're in a gap or at a command
                                        self.complete_commands(&working_set, new_span, offset, true)
                                    } else {
                                        vec![]
                                    };

                                let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
                                    match d.as_string() {
                                        Ok(s) => s,
                                        Err(_) => "".to_string(),
                                    }
                                } else {
                                    "".to_string()
                                };

                                let preceding_byte = if new_span.start > offset {
                                    working_set
                                        .get_span_contents(Span {
                                            start: new_span.start - 1,
                                            end: new_span.start,
                                        })
                                        .to_vec()
                                } else {
                                    vec![]
                                };
                                // let prefix = working_set.get_span_contents(flat.0);
                                let prefix = String::from_utf8_lossy(&prefix).to_string();
                                let output = file_path_completion(new_span, &prefix, &cwd)
                                    .into_iter()
                                    .map(move |x| {
                                        if flat_idx == 0 {
                                            // We're in the command position
                                            if x.1.starts_with('"')
                                                && !matches!(preceding_byte.get(0), Some(b'^'))
                                            {
                                                let trimmed = trim_quotes(x.1.as_bytes());
                                                let trimmed =
                                                    String::from_utf8_lossy(trimmed).to_string();
                                                let expanded =
                                                    nu_path::canonicalize_with(trimmed, &cwd);

                                                if let Ok(expanded) = expanded {
                                                    if is_executable::is_executable(expanded) {
                                                        (x.0, format!("^{}", x.1))
                                                    } else {
                                                        (x.0, x.1)
                                                    }
                                                } else {
                                                    (x.0, x.1)
                                                }
                                            } else {
                                                (x.0, x.1)
                                            }
                                        } else {
                                            (x.0, x.1)
                                        }
                                    })
                                    .map(move |x| {
                                        (
                                            reedline::Span {
                                                start: x.0.start - offset,
                                                end: x.0.end - offset,
                                            },
                                            x.1,
                                        )
                                    })
                                    .chain(subcommands.into_iter())
                                    .chain(commands.into_iter())
                                    .collect::<Vec<_>>();
                                //output.dedup_by(|a, b| a.1 == b.1);

                                return output;
                            }
                        }
                    }
                }
            }
        }

        vec![]
    }
}

impl Completer for NuCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<(reedline::Span, String)> {
        let mut output = self.completion_helper(line, pos);

        output.sort_by(|a, b| a.1.cmp(&b.1));

        output
    }
}

fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
) -> Vec<(nu_protocol::Span, String)> {
    use std::path::{is_separator, Path};

    let partial = partial.replace("\"", "");

    let (base_dir_name, partial) = {
        // If partial is only a word we want to search in the current dir
        let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", &partial));
        // On windows, this standardizes paths to use \
        let mut base = base.replace(is_separator, &SEP.to_string());

        // rsplit_once removes the separator
        base.push(SEP);
        (base, rest)
    };

    let base_dir = nu_path::expand_path_with(&base_dir_name, cwd);
    // This check is here as base_dir.read_dir() with base_dir == "" will open the current dir
    // which we don't want in this case (if we did, base_dir would already be ".")
    if base_dir == Path::new("") {
        return Vec::new();
    }

    if let Ok(result) = base_dir.read_dir() {
        result
            .filter_map(|entry| {
                entry.ok().and_then(|entry| {
                    let mut file_name = entry.file_name().to_string_lossy().into_owned();
                    if matches(partial, &file_name) {
                        let mut path = format!("{}{}", base_dir_name, file_name);
                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        if path.contains(' ') {
                            path = format!("\"{}\"", path);
                        }

                        Some((span, path))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

fn matches(partial: &str, from: &str) -> bool {
    from.to_ascii_lowercase()
        .starts_with(&partial.to_ascii_lowercase())
}
