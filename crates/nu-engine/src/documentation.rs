use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, SyntaxShape, Value,
};
use std::{collections::HashMap, fmt::Write};

use crate::eval_call;

pub fn get_full_help(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
    is_parser_keyword: bool,
) -> String {
    let config = engine_state.get_config();
    let doc_config = DocumentationConfig {
        no_subcommands: false,
        no_color: !config.use_ansi_coloring,
        brief: false,
    };
    get_documentation(
        sig,
        examples,
        engine_state,
        stack,
        &doc_config,
        is_parser_keyword,
    )
}

#[derive(Default)]
struct DocumentationConfig {
    no_subcommands: bool,
    no_color: bool,
    brief: bool,
}

// Utility returns nu-highlighted string
fn nu_highlight_string(code_string: &str, engine_state: &EngineState, stack: &mut Stack) -> String {
    if let Some(highlighter) = engine_state.find_decl(b"nu-highlight", &[]) {
        let decl = engine_state.get_decl(highlighter);

        if let Ok(output) = decl.run(
            engine_state,
            stack,
            &Call::new(Span::unknown()),
            Value::string(code_string, Span::unknown()).into_pipeline_data(),
        ) {
            let result = output.into_value(Span::unknown());
            if let Ok(s) = result.as_string() {
                return s; // successfully highlighted string
            }
        }
    }
    code_string.to_string()
}

#[allow(clippy::cognitive_complexity)]
fn get_documentation(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
    config: &DocumentationConfig,
    is_parser_keyword: bool,
) -> String {
    // Create ansi colors
    //todo make these configurable -- pull from enginestate.config
    const G: &str = "\x1b[32m"; // green
    const C: &str = "\x1b[36m"; // cyan
                                // was const BB: &str = "\x1b[1;34m"; // bold blue
    const BB: &str = "\x1b[94m"; // light blue (nobold, should be bolding the *names*)
    const RESET: &str = "\x1b[0m"; // reset

    let cmd_name = &sig.name;
    let mut long_desc = String::new();

    let usage = &sig.usage;
    if !usage.is_empty() {
        long_desc.push_str(usage);
        long_desc.push_str("\n\n");
    }

    let extra_usage = if config.brief { "" } else { &sig.extra_usage };
    if !extra_usage.is_empty() {
        long_desc.push_str(extra_usage);
        long_desc.push_str("\n\n");
    }

    let mut subcommands = vec![];
    if !config.no_subcommands {
        let signatures = engine_state
            .get_signatures(true)
            .iter()
            .filter(|sig| {
                let is_sub_command = sig.name.starts_with(&format!("{cmd_name} "));
                let has_been_removed = matches!(sig.category, Category::Removed);

                // Don't display removed/deprecated commands in the Subcommands list
                is_sub_command && !has_been_removed
            })
            .map(|sig| sig.clone())
            .collect::<Vec<Signature>>();

        let max_width = signatures
            .iter()
            .map(|sig| sig.name.len())
            .max()
            .unwrap_or(0);

        for sig in signatures {
            subcommands.push(format!(
                "  {C}{:<width$}{RESET} - {}",
                sig.name,
                sig.usage,
                width = max_width
            ));
        }
    }

    if !sig.search_terms.is_empty() {
        let text = format!(
            "{G}Search terms{RESET}: {C}{}{}\n\n",
            sig.search_terms.join(", "),
            RESET
        );
        let _ = write!(long_desc, "{text}");
    }

    let text = format!("{}Usage{}:\n  > {}\n", G, RESET, sig.call_signature());
    let _ = write!(long_desc, "{text}");

    if !subcommands.is_empty() {
        let _ = write!(long_desc, "\n{G}Subcommands{RESET}:\n");
        subcommands.sort();
        long_desc.push_str(&subcommands.join("\n"));
        long_desc.push('\n');
    }

    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(sig, |v| {
            nu_highlight_string(
                &v.into_string_parsable(", ", &engine_state.config),
                engine_state,
                stack,
            )
        }))
    }

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        let _ = write!(long_desc, "\n{G}Parameters{RESET}:\n");
        for positional in &sig.required_positional {
            let text = match &positional.shape {
                SyntaxShape::Keyword(kw, shape) => {
                    format!(
                        "  {C}\"{}\" + {RESET}<{BB}{}{RESET}>: {}",
                        String::from_utf8_lossy(kw),
                        document_shape(*shape.clone()),
                        positional.desc
                    )
                }
                _ => {
                    format!(
                        "  {C}{}{RESET} <{BB}{}{RESET}>: {}",
                        positional.name,
                        document_shape(positional.shape.clone()),
                        positional.desc
                    )
                }
            };
            let _ = writeln!(long_desc, "{text}");
        }
        for positional in &sig.optional_positional {
            let text = match &positional.shape {
                SyntaxShape::Keyword(kw, shape) => {
                    format!(
                        "  {C}\"{}\" + {RESET}<{BB}{}{RESET}>: {} (optional)",
                        String::from_utf8_lossy(kw),
                        document_shape(*shape.clone()),
                        positional.desc
                    )
                }
                _ => {
                    let opt_suffix = if let Some(value) = &positional.default_value {
                        format!(
                            " (optional, default: {})",
                            nu_highlight_string(
                                &value.into_string_parsable(", ", &engine_state.config),
                                engine_state,
                                stack
                            )
                        )
                    } else {
                        (" (optional)").to_string()
                    };

                    format!(
                        "  {C}{}{RESET} <{BB}{}{RESET}>: {}{}",
                        positional.name,
                        document_shape(positional.shape.clone()),
                        positional.desc,
                        opt_suffix,
                    )
                }
            };
            let _ = writeln!(long_desc, "{text}");
        }

        if let Some(rest_positional) = &sig.rest_positional {
            let text = format!(
                "  ...{C}{}{RESET} <{BB}{}{RESET}>: {}",
                rest_positional.name,
                document_shape(rest_positional.shape.clone()),
                rest_positional.desc
            );
            let _ = writeln!(long_desc, "{text}");
        }
    }

    if !is_parser_keyword && !sig.input_output_types.is_empty() {
        if let Some(decl_id) = engine_state.find_decl(b"table", &[]) {
            // FIXME: we may want to make this the span of the help command in the future
            let span = Span::unknown();
            let mut vals = vec![];
            for (input, output) in &sig.input_output_types {
                vals.push(Value::Record {
                    cols: vec!["input".into(), "output".into()],
                    vals: vec![
                        Value::string(input.to_string(), span),
                        Value::string(output.to_string(), span),
                    ],
                    span,
                });
            }

            let mut caller_stack = Stack::new();
            if let Ok(result) = eval_call(
                engine_state,
                &mut caller_stack,
                &Call {
                    decl_id,
                    head: span,
                    arguments: vec![],
                    redirect_stdout: true,
                    redirect_stderr: true,
                    parser_info: HashMap::new(),
                },
                PipelineData::Value(Value::List { vals, span }, None),
            ) {
                if let Ok((str, ..)) = result.collect_string_strict(span) {
                    let _ = writeln!(long_desc, "\n{G}Input/output types{RESET}:");
                    for line in str.lines() {
                        let _ = writeln!(long_desc, "  {line}");
                    }
                }
            }
        }
    }

    if !examples.is_empty() {
        let _ = write!(long_desc, "\n{G}Examples{RESET}:");
    }

    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_color {
            let _ = write!(long_desc, "\n  > {}\n", example.example);
        } else if let Some(highlighter) = engine_state.find_decl(b"nu-highlight", &[]) {
            let decl = engine_state.get_decl(highlighter);

            match decl.run(
                engine_state,
                stack,
                &Call::new(Span::unknown()),
                Value::string(example.example, Span::unknown()).into_pipeline_data(),
            ) {
                Ok(output) => {
                    let result = output.into_value(Span::unknown());
                    match result.as_string() {
                        Ok(s) => {
                            let _ = write!(long_desc, "\n  > {s}\n");
                        }
                        _ => {
                            let _ = write!(long_desc, "\n  > {}\n", example.example);
                        }
                    }
                }
                Err(_) => {
                    let _ = write!(long_desc, "\n  > {}\n", example.example);
                }
            }
        } else {
            let _ = write!(long_desc, "\n  > {}\n", example.example);
        }

        if let Some(result) = &example.result {
            let table = engine_state
                .find_decl("table".as_bytes(), &[])
                .and_then(|decl_id| {
                    engine_state
                        .get_decl(decl_id)
                        .run(
                            engine_state,
                            stack,
                            &Call::new(Span::new(0, 0)),
                            PipelineData::Value(result.clone(), None),
                        )
                        .ok()
                });

            for item in table.into_iter().flatten() {
                let _ = writeln!(
                    long_desc,
                    "  {}",
                    item.into_string("", engine_state.get_config())
                        .replace('\n', "\n  ")
                        .trim()
                );
            }
        }
    }

    long_desc.push('\n');

    if config.no_color {
        nu_utils::strip_ansi_string_likely(long_desc)
    } else {
        long_desc
    }
}

// document shape helps showing more useful information
pub fn document_shape(shape: SyntaxShape) -> SyntaxShape {
    match shape {
        SyntaxShape::Custom(inner_shape, _) => *inner_shape,
        _ => shape,
    }
}

pub fn get_flags_section<F>(
    signature: &Signature,
    mut value_formatter: F, // format default Value (because some calls cant access config or nu-highlight)
) -> String
where
    F: FnMut(&nu_protocol::Value) -> String,
{
    //todo make these configurable -- pull from enginestate.config
    const G: &str = "\x1b[32m"; // green
    const C: &str = "\x1b[36m"; // cyan
                                // was const BB: &str = "\x1b[1;34m"; // bold blue
    const BB: &str = "\x1b[94m"; // light blue (nobold, should be bolding the *names*)
    const RESET: &str = "\x1b[0m"; // reset
    const D: &str = "\x1b[39m"; // default

    let mut long_desc = String::new();
    let _ = write!(long_desc, "\n{G}Flags{RESET}:\n");
    for flag in &signature.named {
        let default_str = if let Some(value) = &flag.default_value {
            format!(" (default: {BB}{}{RESET})", &value_formatter(value))
        } else {
            "".to_string()
        };

        let msg = if let Some(arg) = &flag.arg {
            if let Some(short) = flag.short {
                if flag.required {
                    format!(
                        "  {C}-{}{}{RESET} (required parameter) {:?} - {}{}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {C}--{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc,
                        default_str,
                    )
                } else {
                    format!(
                        "  {C}-{}{}{RESET} <{BB}{:?}{RESET}> - {}{}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {C}--{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc,
                        default_str,
                    )
                }
            } else if flag.required {
                format!(
                    "  {C}--{}{RESET} (required parameter) <{BB}{:?}{RESET}> - {}{}\n",
                    flag.long, arg, flag.desc, default_str,
                )
            } else {
                format!(
                    "  {C}--{}{RESET} <{BB}{:?}{RESET}> - {}{}\n",
                    flag.long, arg, flag.desc, default_str,
                )
            }
        } else if let Some(short) = flag.short {
            if flag.required {
                format!(
                    "  {C}-{}{}{RESET} (required parameter) - {}{}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {C}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc,
                    default_str,
                )
            } else {
                format!(
                    "  {C}-{}{}{RESET} - {}{}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {C}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc,
                    default_str
                )
            }
        } else if flag.required {
            format!(
                "  {C}--{}{RESET} (required parameter) - {}{}\n",
                flag.long, flag.desc, default_str,
            )
        } else {
            format!("  {C}--{}{RESET} - {}\n", flag.long, flag.desc)
        };
        long_desc.push_str(&msg);
    }
    long_desc
}
