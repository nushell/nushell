use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    Example, IntoPipelineData, Signature, Span, SyntaxShape, Value,
};
use std::fmt::Write;

pub fn get_full_help(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    let config = engine_state.get_config();
    let doc_config = DocumentationConfig {
        no_subcommands: false,
        no_color: !config.use_ansi_coloring,
        brief: false,
    };
    get_documentation(sig, examples, engine_state, stack, &doc_config)
}

#[derive(Default)]
struct DocumentationConfig {
    no_subcommands: bool,
    no_color: bool,
    brief: bool,
}

#[allow(clippy::cognitive_complexity)]
fn get_documentation(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
    config: &DocumentationConfig,
) -> String {
    // Create ansi colors
    const G: &str = "\x1b[32m"; // green
    const C: &str = "\x1b[36m"; // cyan
    const BB: &str = "\x1b[1;34m"; // bold blue
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
        let signatures = engine_state.get_signatures(true);
        for sig in signatures {
            if sig.name.starts_with(&format!("{} ", cmd_name)) {
                subcommands.push(format!("  {C}{}{RESET} - {}", sig.name, sig.usage));
            }
        }
    }

    if !sig.search_terms.is_empty() {
        let text = format!(
            "{G}Search terms{RESET}: {C}{}{}\n\n",
            sig.search_terms.join(", "),
            RESET
        );
        let _ = write!(long_desc, "{}", text);
    }

    let text = format!("{}Usage{}:\n  > {}\n", G, RESET, sig.call_signature());
    let _ = write!(long_desc, "{}", text);

    if !subcommands.is_empty() {
        let _ = write!(long_desc, "\n{G}Subcommands{RESET}:\n");
        subcommands.sort();
        long_desc.push_str(&subcommands.join("\n"));
        long_desc.push('\n');
    }

    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(sig))
    }

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        let _ = write!(long_desc, "\n{G}Parameters{RESET}:\n");
        for positional in &sig.required_positional {
            let text = format!(
                "  {C}{}{RESET} <{BB}{:?}{RESET}>: {}",
                positional.name,
                document_shape(positional.shape.clone()),
                positional.desc
            );
            let _ = writeln!(long_desc, "{}", text);
        }
        for positional in &sig.optional_positional {
            let text = format!(
                "  (optional) {C}{}{RESET} <{BB}{:?}{RESET}>: {}",
                positional.name,
                document_shape(positional.shape.clone()),
                positional.desc
            );
            let _ = writeln!(long_desc, "{}", text);
        }

        if let Some(rest_positional) = &sig.rest_positional {
            let text = format!(
                "  ...{C}{}{RESET} <{BB}{:?}{RESET}>: {}",
                rest_positional.name,
                document_shape(rest_positional.shape.clone()),
                rest_positional.desc
            );
            let _ = writeln!(long_desc, "{}", text);
        }
    }

    if !examples.is_empty() {
        let _ = write!(long_desc, "\n{}Examples{}:", G, RESET);
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
                &Call::new(Span::new(0, 0)),
                Value::String {
                    val: example.example.to_string(),
                    span: Span { start: 0, end: 0 },
                }
                .into_pipeline_data(),
            ) {
                Ok(output) => {
                    let result = output.into_value(Span { start: 0, end: 0 });
                    match result.as_string() {
                        Ok(s) => {
                            let _ = write!(long_desc, "\n  > {}\n", s);
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

pub fn get_flags_section(signature: &Signature) -> String {
    const G: &str = "\x1b[32m"; // green
    const C: &str = "\x1b[36m"; // cyan
    const BB: &str = "\x1b[1;34m"; // bold blue
    const RESET: &str = "\x1b[0m"; // reset
    const D: &str = "\x1b[39m"; // default

    let mut long_desc = String::new();
    let _ = write!(long_desc, "\n{}Flags{}:\n", G, RESET);
    for flag in &signature.named {
        let msg = if let Some(arg) = &flag.arg {
            if let Some(short) = flag.short {
                if flag.required {
                    format!(
                        "  {C}-{}{}{RESET} (required parameter) {:?} - {}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {C}--{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc
                    )
                } else {
                    format!(
                        "  {C}-{}{}{RESET} <{BB}{:?}{RESET}> - {}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {C}--{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc
                    )
                }
            } else if flag.required {
                format!(
                    "  {C}--{}{RESET} (required parameter) <{BB}{:?}{RESET}> - {}\n",
                    flag.long, arg, flag.desc
                )
            } else {
                format!(
                    "  {C}--{}{RESET} <{BB}{:?}{RESET}> - {}\n",
                    flag.long, arg, flag.desc
                )
            }
        } else if let Some(short) = flag.short {
            if flag.required {
                format!(
                    "  {C}-{}{}{RESET} (required parameter) - {}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {C}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc
                )
            } else {
                format!(
                    "  {C}-{}{}{RESET} - {}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {C}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc
                )
            }
        } else if flag.required {
            format!(
                "  {C}--{}{RESET} (required parameter) - {}\n",
                flag.long, flag.desc
            )
        } else {
            format!("  {C}--{}{RESET} - {}\n", flag.long, flag.desc)
        };
        long_desc.push_str(&msg);
    }
    long_desc
}
