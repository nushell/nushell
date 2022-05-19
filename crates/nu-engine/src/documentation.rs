use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    Example, IntoPipelineData, Signature, Span, Value,
};

pub fn get_full_help(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    get_documentation(
        sig,
        examples,
        engine_state,
        stack,
        &DocumentationConfig::default(),
    )
}

#[derive(Default)]
struct DocumentationConfig {
    no_subcommands: bool,
    //FIXME: add back in color support
    #[allow(dead_code)]
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
                subcommands.push(format!("  {} - {}", sig.name, sig.usage));
            }
        }
    }

    if !sig.search_terms.is_empty() {
        long_desc.push_str(&format!(
            "Search terms: {}\n\n",
            sig.search_terms.join(", ")
        ));
    }

    long_desc.push_str(&format!("Usage:\n  > {}\n", sig.call_signature()));

    if !subcommands.is_empty() {
        long_desc.push_str("\nSubcommands:\n");
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
        long_desc.push_str("\nParameters:\n");
        for positional in &sig.required_positional {
            long_desc.push_str(&format!(
                "  {} <{:?}>: {}\n",
                positional.name, positional.shape, positional.desc
            ));
        }
        for positional in &sig.optional_positional {
            long_desc.push_str(&format!(
                "  (optional) {} <{:?}>: {}\n",
                positional.name, positional.shape, positional.desc
            ));
        }

        if let Some(rest_positional) = &sig.rest_positional {
            long_desc.push_str(&format!(
                "  ...{} <{:?}>: {}\n",
                rest_positional.name, rest_positional.shape, rest_positional.desc
            ));
        }
    }

    if !examples.is_empty() {
        long_desc.push_str("\nExamples:");
    }

    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_color {
            long_desc.push_str(&format!("\n  > {}\n", example.example));
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
                            long_desc.push_str(&format!("\n  > {}\n", s));
                        }
                        _ => {
                            long_desc.push_str(&format!("\n  > {}\n", example.example));
                        }
                    }
                }
                Err(_) => {
                    long_desc.push_str(&format!("\n  > {}\n", example.example));
                }
            }
        } else {
            long_desc.push_str(&format!("\n  > {}\n", example.example));
        }
    }

    long_desc.push('\n');

    long_desc
}

pub fn get_flags_section(signature: &Signature) -> String {
    let mut long_desc = String::new();
    long_desc.push_str("\nFlags:\n");
    for flag in &signature.named {
        let msg = if let Some(arg) = &flag.arg {
            if let Some(short) = flag.short {
                if flag.required {
                    format!(
                        "  -{}{} (required parameter) {:?}\n      {}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!(", --{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc
                    )
                } else {
                    format!(
                        "  -{}{} <{:?}>\n      {}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!(", --{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc
                    )
                }
            } else if flag.required {
                format!(
                    "  --{} (required parameter) <{:?}>\n      {}\n",
                    flag.long, arg, flag.desc
                )
            } else {
                format!("  --{} <{:?}>\n      {}\n", flag.long, arg, flag.desc)
            }
        } else if let Some(short) = flag.short {
            if flag.required {
                format!(
                    "  -{}{} (required parameter)\n      {}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!(", --{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc
                )
            } else {
                format!(
                    "  -{}{}\n      {}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!(", --{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc
                )
            }
        } else if flag.required {
            format!(
                "  --{} (required parameter)\n      {}\n",
                flag.long, flag.desc
            )
        } else {
            format!("  --{}\n      {}\n", flag.long, flag.desc)
        };
        long_desc.push_str(&msg);
    }
    long_desc
}
