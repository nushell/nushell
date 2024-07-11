use crate::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression, RecordItem},
    debugger::WithoutDebug,
    engine::{Command, EngineState, Stack, UNKNOWN_SPAN_ID},
    record, Category, Config, Example, IntoPipelineData, PipelineData, Signature, Span, SpanId,
    Spanned, SyntaxShape, Type, Value,
};
use std::{collections::HashMap, fmt::Write};

pub fn get_full_help(
    command: &dyn Command,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    let config = stack.get_config(engine_state);
    let doc_config = DocumentationConfig {
        no_subcommands: false,
        no_color: !config.use_ansi_coloring,
        brief: false,
    };

    let stack = &mut stack.start_capture();
    let signature = command.signature().update_from_command(command);

    get_documentation(
        &signature,
        &command.examples(),
        engine_state,
        stack,
        &doc_config,
        command.is_keyword(),
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

        let call = Call::new(Span::unknown());

        if let Ok(output) = decl.run(
            engine_state,
            stack,
            &(&call).into(),
            Value::string(code_string, Span::unknown()).into_pipeline_data(),
        ) {
            let result = output.into_value(Span::unknown());
            if let Ok(s) = result.and_then(Value::coerce_into_string) {
                return s; // successfully highlighted string
            }
        }
    }
    code_string.to_string()
}

fn get_documentation(
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
    config: &DocumentationConfig,
    is_parser_keyword: bool,
) -> String {
    let nu_config = stack.get_config(engine_state);

    // Create ansi colors
    //todo make these configurable -- pull from enginestate.config
    let help_section_name: String = get_ansi_color_for_component_or_default(
        engine_state,
        &nu_config,
        "shape_string",
        "\x1b[32m",
    ); // default: green

    let help_subcolor_one: String = get_ansi_color_for_component_or_default(
        engine_state,
        &nu_config,
        "shape_external",
        "\x1b[36m",
    ); // default: cyan
       // was const bb: &str = "\x1b[1;34m"; // bold blue
    let help_subcolor_two: String = get_ansi_color_for_component_or_default(
        engine_state,
        &nu_config,
        "shape_block",
        "\x1b[94m",
    ); // default: light blue (nobold, should be bolding the *names*)

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
            if sig.name.starts_with(&format!("{cmd_name} "))
                // Don't display removed/deprecated commands in the Subcommands list
                    && !matches!(sig.category, Category::Removed)
            {
                subcommands.push(format!(
                    "  {help_subcolor_one}{}{RESET} - {}",
                    sig.name, sig.usage
                ));
            }
        }
    }

    if !sig.search_terms.is_empty() {
        let text = format!(
            "{help_section_name}Search terms{RESET}: {help_subcolor_one}{}{}\n\n",
            sig.search_terms.join(", "),
            RESET
        );
        let _ = write!(long_desc, "{text}");
    }

    let text = format!(
        "{}Usage{}:\n  > {}\n",
        help_section_name,
        RESET,
        sig.call_signature()
    );
    let _ = write!(long_desc, "{text}");

    if !subcommands.is_empty() {
        let _ = write!(long_desc, "\n{help_section_name}Subcommands{RESET}:\n");
        subcommands.sort();
        long_desc.push_str(&subcommands.join("\n"));
        long_desc.push('\n');
    }

    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(
            Some(engine_state),
            Some(&nu_config),
            sig,
            |v| nu_highlight_string(&v.to_parsable_string(", ", &nu_config), engine_state, stack),
        ))
    }

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        let _ = write!(long_desc, "\n{help_section_name}Parameters{RESET}:\n");
        for positional in &sig.required_positional {
            let text = match &positional.shape {
                SyntaxShape::Keyword(kw, shape) => {
                    format!(
                        "  {help_subcolor_one}\"{}\" + {RESET}<{help_subcolor_two}{}{RESET}>: {}",
                        String::from_utf8_lossy(kw),
                        document_shape(*shape.clone()),
                        positional.desc
                    )
                }
                _ => {
                    format!(
                        "  {help_subcolor_one}{}{RESET} <{help_subcolor_two}{}{RESET}>: {}",
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
                        "  {help_subcolor_one}\"{}\" + {RESET}<{help_subcolor_two}{}{RESET}>: {} (optional)",
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
                                &value.to_parsable_string(", ", &nu_config),
                                engine_state,
                                stack
                            )
                        )
                    } else {
                        (" (optional)").to_string()
                    };

                    format!(
                        "  {help_subcolor_one}{}{RESET} <{help_subcolor_two}{}{RESET}>: {}{}",
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
                "  ...{help_subcolor_one}{}{RESET} <{help_subcolor_two}{}{RESET}>: {}",
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
                vals.push(Value::record(
                    record! {
                        "input" => Value::string(input.to_string(), span),
                        "output" => Value::string(output.to_string(), span),
                    },
                    span,
                ));
            }

            let caller_stack = &mut Stack::new().capture();
            if let Ok(result) = eval_call::<WithoutDebug>(
                engine_state,
                caller_stack,
                &Call {
                    decl_id,
                    head: span,
                    arguments: vec![],
                    parser_info: HashMap::new(),
                },
                PipelineData::Value(Value::list(vals, span), None),
            ) {
                if let Ok((str, ..)) = result.collect_string_strict(span) {
                    let _ = writeln!(long_desc, "\n{help_section_name}Input/output types{RESET}:");
                    for line in str.lines() {
                        let _ = writeln!(long_desc, "  {line}");
                    }
                }
            }
        }
    }

    if !examples.is_empty() {
        let _ = write!(long_desc, "\n{help_section_name}Examples{RESET}:");
    }

    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_color {
            let _ = write!(long_desc, "\n  > {}\n", example.example);
        } else if let Some(highlighter) = engine_state.find_decl(b"nu-highlight", &[]) {
            let decl = engine_state.get_decl(highlighter);
            let call = Call::new(Span::unknown());

            match decl.run(
                engine_state,
                stack,
                &(&call).into(),
                Value::string(example.example, Span::unknown()).into_pipeline_data(),
            ) {
                Ok(output) => {
                    let result = output.into_value(Span::unknown());
                    match result.and_then(Value::coerce_into_string) {
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
            let mut table_call = Call::new(Span::unknown());
            if example.example.ends_with("--collapse") {
                // collapse the result
                table_call.add_named((
                    Spanned {
                        item: "collapse".to_string(),
                        span: Span::unknown(),
                    },
                    None,
                    None,
                ))
            } else {
                // expand the result
                table_call.add_named((
                    Spanned {
                        item: "expand".to_string(),
                        span: Span::unknown(),
                    },
                    None,
                    None,
                ))
            }
            let table = engine_state
                .find_decl("table".as_bytes(), &[])
                .and_then(|decl_id| {
                    engine_state
                        .get_decl(decl_id)
                        .run(
                            engine_state,
                            stack,
                            &(&table_call).into(),
                            PipelineData::Value(result.clone(), None),
                        )
                        .ok()
                });

            for item in table.into_iter().flatten() {
                let _ = writeln!(
                    long_desc,
                    "  {}",
                    item.to_expanded_string("", &nu_config)
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

fn get_ansi_color_for_component_or_default(
    engine_state: &EngineState,
    nu_config: &Config,
    theme_component: &str,
    default: &str,
) -> String {
    if let Some(color) = &nu_config.color_config.get(theme_component) {
        let caller_stack = &mut Stack::new().capture();
        let span = Span::unknown();
        let span_id = UNKNOWN_SPAN_ID;

        let argument_opt = get_argument_for_color_value(nu_config, color, span, span_id);

        // Call ansi command using argument
        if let Some(argument) = argument_opt {
            if let Some(decl_id) = engine_state.find_decl(b"ansi", &[]) {
                if let Ok(result) = eval_call::<WithoutDebug>(
                    engine_state,
                    caller_stack,
                    &Call {
                        decl_id,
                        head: span,
                        arguments: vec![argument],
                        parser_info: HashMap::new(),
                    },
                    PipelineData::Empty,
                ) {
                    if let Ok((str, ..)) = result.collect_string_strict(span) {
                        return str;
                    }
                }
            }
        }
    }

    default.to_string()
}

fn get_argument_for_color_value(
    nu_config: &Config,
    color: &Value,
    span: Span,
    span_id: SpanId,
) -> Option<Argument> {
    match color {
        Value::Record { val, .. } => {
            let record_exp: Vec<RecordItem> = (**val)
                .iter()
                .map(|(k, v)| {
                    RecordItem::Pair(
                        Expression::new_existing(
                            Expr::String(k.clone()),
                            span,
                            span_id,
                            Type::String,
                        ),
                        Expression::new_existing(
                            Expr::String(v.clone().to_expanded_string("", nu_config)),
                            span,
                            span_id,
                            Type::String,
                        ),
                    )
                })
                .collect();

            Some(Argument::Positional(Expression::new_existing(
                Expr::Record(record_exp),
                Span::unknown(),
                UNKNOWN_SPAN_ID,
                Type::Record(
                    [
                        ("fg".to_string(), Type::String),
                        ("attr".to_string(), Type::String),
                    ]
                    .into(),
                ),
            )))
        }
        Value::String { val, .. } => Some(Argument::Positional(Expression::new_existing(
            Expr::String(val.clone()),
            Span::unknown(),
            UNKNOWN_SPAN_ID,
            Type::String,
        ))),
        _ => None,
    }
}

// document shape helps showing more useful information
pub fn document_shape(shape: SyntaxShape) -> SyntaxShape {
    match shape {
        SyntaxShape::CompleterWrapper(inner_shape, _) => *inner_shape,
        _ => shape,
    }
}

pub fn get_flags_section<F>(
    engine_state_opt: Option<&EngineState>,
    nu_config_opt: Option<&Config>,
    signature: &Signature,
    mut value_formatter: F, // format default Value (because some calls cant access config or nu-highlight)
) -> String
where
    F: FnMut(&nu_protocol::Value) -> String,
{
    //todo make these configurable -- pull from enginestate.config
    let help_section_name: String;
    let help_subcolor_one: String;
    let help_subcolor_two: String;

    // Sometimes we want to get the flags without engine_state
    // For example, in nu-plugin. In that case, we fall back on default values
    if let Some(engine_state) = engine_state_opt {
        let nu_config = nu_config_opt.unwrap_or_else(|| engine_state.get_config());
        help_section_name = get_ansi_color_for_component_or_default(
            engine_state,
            nu_config,
            "shape_string",
            "\x1b[32m",
        ); // default: green
        help_subcolor_one = get_ansi_color_for_component_or_default(
            engine_state,
            nu_config,
            "shape_external",
            "\x1b[36m",
        ); // default: cyan
           // was const bb: &str = "\x1b[1;34m"; // bold blue
        help_subcolor_two = get_ansi_color_for_component_or_default(
            engine_state,
            nu_config,
            "shape_block",
            "\x1b[94m",
        );
    // default: light blue (nobold, should be bolding the *names*)
    } else {
        help_section_name = "\x1b[32m".to_string();
        help_subcolor_one = "\x1b[36m".to_string();
        help_subcolor_two = "\x1b[94m".to_string();
    }

    const RESET: &str = "\x1b[0m"; // reset
    const D: &str = "\x1b[39m"; // default

    let mut long_desc = String::new();
    let _ = write!(long_desc, "\n{help_section_name}Flags{RESET}:\n");
    for flag in &signature.named {
        let default_str = if let Some(value) = &flag.default_value {
            format!(
                " (default: {help_subcolor_two}{}{RESET})",
                &value_formatter(value)
            )
        } else {
            "".to_string()
        };

        let msg = if let Some(arg) = &flag.arg {
            if let Some(short) = flag.short {
                if flag.required {
                    format!(
                        "  {help_subcolor_one}-{}{}{RESET} (required parameter) {:?} - {}{}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {help_subcolor_one}--{}", flag.long)
                        } else {
                            "".into()
                        },
                        arg,
                        flag.desc,
                        default_str,
                    )
                } else {
                    format!(
                        "  {help_subcolor_one}-{}{}{RESET} <{help_subcolor_two}{:?}{RESET}> - {}{}\n",
                        short,
                        if !flag.long.is_empty() {
                            format!("{D},{RESET} {help_subcolor_one}--{}", flag.long)
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
                    "  {help_subcolor_one}--{}{RESET} (required parameter) <{help_subcolor_two}{:?}{RESET}> - {}{}\n",
                    flag.long, arg, flag.desc, default_str,
                )
            } else {
                format!(
                    "  {help_subcolor_one}--{}{RESET} <{help_subcolor_two}{:?}{RESET}> - {}{}\n",
                    flag.long, arg, flag.desc, default_str,
                )
            }
        } else if let Some(short) = flag.short {
            if flag.required {
                format!(
                    "  {help_subcolor_one}-{}{}{RESET} (required parameter) - {}{}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {help_subcolor_one}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc,
                    default_str,
                )
            } else {
                format!(
                    "  {help_subcolor_one}-{}{}{RESET} - {}{}\n",
                    short,
                    if !flag.long.is_empty() {
                        format!("{D},{RESET} {help_subcolor_one}--{}", flag.long)
                    } else {
                        "".into()
                    },
                    flag.desc,
                    default_str
                )
            }
        } else if flag.required {
            format!(
                "  {help_subcolor_one}--{}{RESET} (required parameter) - {}{}\n",
                flag.long, flag.desc, default_str,
            )
        } else {
            format!(
                "  {help_subcolor_one}--{}{RESET} - {}\n",
                flag.long, flag.desc
            )
        };
        long_desc.push_str(&msg);
    }
    long_desc
}
