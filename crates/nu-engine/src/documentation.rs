use crate::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression, RecordItem},
    debugger::WithoutDebug,
    engine::{Command, EngineState, Stack, UNKNOWN_SPAN_ID},
    record, Category, Config, Example, IntoPipelineData, PipelineData, PositionalArg, Signature,
    Span, SpanId, Spanned, SyntaxShape, Type, Value,
};
use std::{collections::HashMap, fmt::Write};
use terminal_size::{Height, Width};

/// ANSI style reset
const RESET: &str = "\x1b[0m";
/// ANSI set default color (as set in the terminal)
const DEFAULT_COLOR: &str = "\x1b[39m";

pub fn get_full_help(
    command: &dyn Command,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    // Precautionary step to capture any command output generated during this operation. We
    // internally call several commands (`table`, `ansi`, `nu-highlight`) and get their
    // `PipelineData` using this `Stack`, any other output should not be redirected like the main
    // execution.
    let stack = &mut stack.start_collect_value();

    let signature = engine_state
        .get_signature(command)
        .update_from_command(command);

    get_documentation(
        &signature,
        &command.examples(),
        engine_state,
        stack,
        command.is_keyword(),
    )
}

/// Syntax highlight code using the `nu-highlight` command if available
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
    is_parser_keyword: bool,
) -> String {
    let nu_config = stack.get_config(engine_state);

    // Create ansi colors
    let mut help_style = HelpStyle::default();
    help_style.update_from_config(engine_state, &nu_config);
    let help_section_name = &help_style.section_name;
    let help_subcolor_one = &help_style.subcolor_one;

    let cmd_name = &sig.name;
    let mut long_desc = String::new();

    let desc = &sig.description;
    if !desc.is_empty() {
        long_desc.push_str(desc);
        long_desc.push_str("\n\n");
    }

    let extra_desc = &sig.extra_description;
    if !extra_desc.is_empty() {
        long_desc.push_str(extra_desc);
        long_desc.push_str("\n\n");
    }

    if !sig.search_terms.is_empty() {
        let _ = write!(
            long_desc,
            "{help_section_name}Search terms{RESET}: {help_subcolor_one}{}{RESET}\n\n",
            sig.search_terms.join(", "),
        );
    }

    let _ = write!(
        long_desc,
        "{help_section_name}Usage{RESET}:\n  > {}\n",
        sig.call_signature()
    );

    // TODO: improve the subcommand name resolution
    // issues:
    // - Aliases are included
    //   - https://github.com/nushell/nushell/issues/11657
    // - Subcommands are included violating module scoping
    //   - https://github.com/nushell/nushell/issues/11447
    //   - https://github.com/nushell/nushell/issues/11625
    let mut subcommands = vec![];
    let signatures = engine_state.get_signatures(true);
    for sig in signatures {
        // Don't display removed/deprecated commands in the Subcommands list
        if sig.name.starts_with(&format!("{cmd_name} "))
            && !matches!(sig.category, Category::Removed)
        {
            subcommands.push(format!(
                "  {help_subcolor_one}{}{RESET} - {}",
                sig.name, sig.description
            ));
        }
    }

    if !subcommands.is_empty() {
        let _ = write!(long_desc, "\n{help_section_name}Subcommands{RESET}:\n");
        subcommands.sort();
        long_desc.push_str(&subcommands.join("\n"));
        long_desc.push('\n');
    }

    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(sig, &help_style, |v| {
            nu_highlight_string(&v.to_parsable_string(", ", &nu_config), engine_state, stack)
        }))
    }

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        let _ = write!(long_desc, "\n{help_section_name}Parameters{RESET}:\n");
        for positional in &sig.required_positional {
            write_positional(
                &mut long_desc,
                positional,
                PositionalKind::Required,
                &help_style,
                &nu_config,
                engine_state,
                stack,
            );
        }
        for positional in &sig.optional_positional {
            write_positional(
                &mut long_desc,
                positional,
                PositionalKind::Optional,
                &help_style,
                &nu_config,
                engine_state,
                stack,
            );
        }

        if let Some(rest_positional) = &sig.rest_positional {
            write_positional(
                &mut long_desc,
                rest_positional,
                PositionalKind::Rest,
                &help_style,
                &nu_config,
                engine_state,
                stack,
            );
        }
    }

    fn get_term_width() -> usize {
        if let Some((Width(w), Height(_))) = terminal_size::terminal_size() {
            w as usize
        } else {
            80
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

            let caller_stack = &mut Stack::new().collect_value();
            if let Ok(result) = eval_call::<WithoutDebug>(
                engine_state,
                caller_stack,
                &Call {
                    decl_id,
                    head: span,
                    arguments: vec![Argument::Named((
                        Spanned {
                            item: "width".to_string(),
                            span: Span::unknown(),
                        },
                        None,
                        Some(Expression::new_unknown(
                            Expr::Int(get_term_width() as i64 - 2), // padding, see below
                            Span::unknown(),
                            Type::Int,
                        )),
                    ))],
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

        if !nu_config.use_ansi_coloring {
            let _ = write!(long_desc, "\n  > {}\n", example.example);
        } else {
            let code_string = nu_highlight_string(example.example, engine_state, stack);
            let _ = write!(long_desc, "\n  > {code_string}\n");
        };

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
            table_call.add_named((
                Spanned {
                    item: "width".to_string(),
                    span: Span::unknown(),
                },
                None,
                Some(Expression::new_unknown(
                    Expr::Int(get_term_width() as i64 - 2),
                    Span::unknown(),
                    Type::Int,
                )),
            ));

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

    if !nu_config.use_ansi_coloring {
        nu_utils::strip_ansi_string_likely(long_desc)
    } else {
        long_desc
    }
}

fn update_ansi_from_config(
    ansi_code: &mut String,
    engine_state: &EngineState,
    nu_config: &Config,
    theme_component: &str,
) {
    if let Some(color) = &nu_config.color_config.get(theme_component) {
        let caller_stack = &mut Stack::new().collect_value();
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
                        *ansi_code = str;
                    }
                }
            }
        }
    }
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

/// Contains the settings for ANSI colors in help output
///
/// By default contains a fixed set of (4-bit) colors
///
/// Can reflect configuration using [`HelpStyle::update_from_config`]
pub struct HelpStyle {
    section_name: String,
    subcolor_one: String,
    subcolor_two: String,
}

impl Default for HelpStyle {
    fn default() -> Self {
        HelpStyle {
            // default: green
            section_name: "\x1b[32m".to_string(),
            // default: cyan
            subcolor_one: "\x1b[36m".to_string(),
            // default: light blue
            subcolor_two: "\x1b[94m".to_string(),
        }
    }
}

impl HelpStyle {
    /// Pull colors from the [`Config`]
    ///
    /// Uses some arbitrary `shape_*` settings, assuming they are well visible in the terminal theme.
    ///
    /// Implementation detail: currently executes `ansi` command internally thus requiring the
    /// [`EngineState`] for execution.
    /// See <https://github.com/nushell/nushell/pull/10623> for details
    pub fn update_from_config(&mut self, engine_state: &EngineState, nu_config: &Config) {
        update_ansi_from_config(
            &mut self.section_name,
            engine_state,
            nu_config,
            "shape_string",
        );
        update_ansi_from_config(
            &mut self.subcolor_one,
            engine_state,
            nu_config,
            "shape_external",
        );
        update_ansi_from_config(
            &mut self.subcolor_two,
            engine_state,
            nu_config,
            "shape_block",
        );
    }
}

/// Make syntax shape presentable by stripping custom completer info
fn document_shape(shape: &SyntaxShape) -> &SyntaxShape {
    match shape {
        SyntaxShape::CompleterWrapper(inner_shape, _) => inner_shape,
        _ => shape,
    }
}

#[derive(PartialEq)]
enum PositionalKind {
    Required,
    Optional,
    Rest,
}

fn write_positional(
    long_desc: &mut String,
    positional: &PositionalArg,
    arg_kind: PositionalKind,
    help_style: &HelpStyle,
    nu_config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) {
    let help_subcolor_one = &help_style.subcolor_one;
    let help_subcolor_two = &help_style.subcolor_two;

    // Indentation
    long_desc.push_str("  ");
    if arg_kind == PositionalKind::Rest {
        long_desc.push_str("...");
    }
    match &positional.shape {
        SyntaxShape::Keyword(kw, shape) => {
            let _ = write!(
                long_desc,
                "{help_subcolor_one}\"{}\" + {RESET}<{help_subcolor_two}{}{RESET}>",
                String::from_utf8_lossy(kw),
                document_shape(shape),
            );
        }
        _ => {
            let _ = write!(
                long_desc,
                "{help_subcolor_one}{}{RESET} <{help_subcolor_two}{}{RESET}>",
                positional.name,
                document_shape(&positional.shape),
            );
        }
    };
    if !positional.desc.is_empty() || arg_kind == PositionalKind::Optional {
        let _ = write!(long_desc, ": {}", positional.desc);
    }
    if arg_kind == PositionalKind::Optional {
        if let Some(value) = &positional.default_value {
            let _ = write!(
                long_desc,
                " (optional, default: {})",
                nu_highlight_string(
                    &value.to_parsable_string(", ", nu_config),
                    engine_state,
                    stack
                )
            );
        } else {
            long_desc.push_str(" (optional)");
        };
    }
    long_desc.push('\n');
}

pub fn get_flags_section<F>(
    signature: &Signature,
    help_style: &HelpStyle,
    mut value_formatter: F, // format default Value (because some calls cant access config or nu-highlight)
) -> String
where
    F: FnMut(&nu_protocol::Value) -> String,
{
    let help_section_name = &help_style.section_name;
    let help_subcolor_one = &help_style.subcolor_one;
    let help_subcolor_two = &help_style.subcolor_two;

    let mut long_desc = String::new();
    let _ = write!(long_desc, "\n{help_section_name}Flags{RESET}:\n");
    for flag in &signature.named {
        // Indentation
        long_desc.push_str("  ");
        // Short flag shown before long flag
        if let Some(short) = flag.short {
            let _ = write!(long_desc, "{help_subcolor_one}-{}{RESET}", short);
            if !flag.long.is_empty() {
                let _ = write!(long_desc, "{DEFAULT_COLOR},{RESET} ");
            }
        }
        if !flag.long.is_empty() {
            let _ = write!(long_desc, "{help_subcolor_one}--{}{RESET}", flag.long);
        }
        if flag.required {
            long_desc.push_str(" (required parameter)")
        }
        // Type/Syntax shape info
        if let Some(arg) = &flag.arg {
            let _ = write!(
                long_desc,
                " <{help_subcolor_two}{}{RESET}>",
                document_shape(arg)
            );
        }
        if !flag.desc.is_empty() {
            let _ = write!(long_desc, ": {}", flag.desc);
        }
        if let Some(value) = &flag.default_value {
            let _ = write!(long_desc, " (default: {})", &value_formatter(value));
        }
        long_desc.push('\n');
    }
    long_desc
}
