use crate::eval_call;
use fancy_regex::{Captures, Regex};
use nu_protocol::{
    Alias, Category, Config, IntoPipelineData, PipelineData, PositionalArg, Signature, Span,
    SpanId, Spanned, SyntaxShape, Type, Value,
    ast::{Argument, Call, Expr, Expression, RecordItem},
    debugger::WithoutDebug,
    engine::CommandType,
    engine::{Command, EngineState, Stack, UNKNOWN_SPAN_ID},
    record,
};
use nu_utils::terminal_size;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::{Arc, LazyLock},
};

/// ANSI style reset
const RESET: &str = "\x1b[0m";
/// ANSI set default color (as set in the terminal)
const DEFAULT_COLOR: &str = "\x1b[39m";
/// ANSI set default dimmed
const DEFAULT_DIMMED: &str = "\x1b[2;39m";
/// ANSI set default italic
const DEFAULT_ITALIC: &str = "\x1b[3;39m";

pub fn get_full_help(
    command: &dyn Command,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> String {
    // Precautionary step to capture any command output generated during this operation. We
    // internally call several commands (`table`, `ansi`, `nu-highlight`) and get their
    // `PipelineData` using this `Stack`, any other output should not be redirected like the main
    // execution.
    let stack = &mut stack.start_collect_value();

    let nu_config = stack.get_config(engine_state);

    let sig = engine_state
        .get_signature(command)
        .update_from_command(command);

    // Create ansi colors
    let mut help_style = HelpStyle::default();
    help_style.update_from_config(engine_state, &nu_config, head);

    let mut long_desc = String::new();

    let desc = &sig.description;
    if !desc.is_empty() {
        long_desc.push_str(&highlight_code(desc, engine_state, stack, head));
        long_desc.push_str("\n\n");
    }

    let extra_desc = &sig.extra_description;
    if !extra_desc.is_empty() {
        long_desc.push_str(&highlight_code(extra_desc, engine_state, stack, head));
        long_desc.push_str("\n\n");
    }

    match command.command_type() {
        CommandType::Alias => get_alias_documentation(
            &mut long_desc,
            command,
            &help_style,
            engine_state,
            stack,
            head,
        ),
        _ => get_command_documentation(
            &mut long_desc,
            command,
            &sig,
            &nu_config,
            &help_style,
            engine_state,
            stack,
            head,
        ),
    };

    let mut final_help = if !nu_config.use_ansi_coloring.get(engine_state) {
        nu_utils::strip_ansi_string_likely(long_desc)
    } else {
        long_desc
    };

    if let Some(cmd) = command.as_alias().and_then(|alias| alias.command.as_ref()) {
        let nested_help = get_full_help(cmd.as_ref(), engine_state, stack, head);
        if !nested_help.is_empty() {
            final_help.push_str("\n\n");
            final_help.push_str(&nested_help);
        }
    }

    final_help
}

/// Syntax highlight code using the `nu-highlight` command if available
fn try_nu_highlight(
    code_string: &str,
    reject_garbage: bool,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> Option<String> {
    let highlighter = engine_state.find_decl(b"nu-highlight", &[])?;

    let decl = engine_state.get_decl(highlighter);
    let mut call = Call::new(head);
    if reject_garbage {
        call.add_named((
            Spanned {
                item: "reject-garbage".into(),
                span: head,
            },
            None,
            None,
        ));
    }

    decl.run(
        engine_state,
        stack,
        &(&call).into(),
        Value::string(code_string, head).into_pipeline_data(),
    )
    .and_then(|pipe| pipe.into_value(head))
    .and_then(|val| val.coerce_into_string())
    .ok()
}

/// Syntax highlight code using the `nu-highlight` command if available, falling back to the given string
fn nu_highlight_string(
    code_string: &str,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> String {
    try_nu_highlight(code_string, false, engine_state, stack, head)
        .unwrap_or_else(|| code_string.to_string())
}

/// Highlight a code snippet the way [`highlight_code`] highlights text between backticks
fn highlight_inline_code(
    content: &str,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> String {
    // Save current color config
    let config_old = stack.get_config(engine_state);
    let mut config = (*config_old).clone();

    // Style externals and external arguments with fallback style,
    // so nu-highlight styles code which is technically valid syntax,
    // but not an internal command is highlighted with the fallback style
    let code_style = Value::record(
        record! {
            "attr" => Value::string("di", head),
        },
        head,
    );
    let color_config = &mut config.color_config;
    color_config.insert("shape_external".into(), code_style.clone());
    color_config.insert("shape_external_resolved".into(), code_style.clone());
    color_config.insert("shape_externalarg".into(), code_style);

    // Apply config with external argument style
    stack.config = Some(Arc::new(config));

    // Highlight and reject invalid syntax
    let highlighted = try_nu_highlight(content, true, engine_state, stack, head)
        // // Make highlighted string italic
        .map(|text| {
            let resets = text.match_indices(RESET).count();
            // replace resets with reset + italic, so the whole string is italicized, excluding the final reset
            let text = text.replacen(
                RESET,
                &format!("{RESET}{DEFAULT_ITALIC}"),
                resets.saturating_sub(1),
            );
            // start italicized
            format!("{DEFAULT_ITALIC}{text}")
        });

    // Restore original config
    stack.config = Some(config_old);

    // Use fallback style if highlight failed/syntax was invalid
    highlighted.unwrap_or_else(|| highlight_fallback(content))
}

/// Apply fallback code style
fn highlight_fallback(text: &str) -> String {
    format!("{DEFAULT_DIMMED}{DEFAULT_ITALIC}{text}{RESET}")
}

/// Highlight code within backticks
///
/// Will attempt to use nu-highlight, falling back to dimmed and italic on invalid syntax
fn highlight_code<'a>(
    text: &'a str,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> Cow<'a, str> {
    let config = stack.get_config(engine_state);
    if !config.use_ansi_coloring.get(engine_state) {
        return Cow::Borrowed(text);
    }

    // See [`tests::test_code_formatting`] for examples
    static PATTERN: &str = r"(?x)     # verbose mode
        (?<![\p{Letter}\d])    # negative look-behind for alphanumeric: ensure backticks are not directly preceded by letter/number.
        `
        ([^`\n]+?)           # capture characters inside backticks, excluding backticks and newlines. ungreedy.
        `
        (?![\p{Letter}\d])     # negative look-ahead for alphanumeric: ensure backticks are not directly followed by letter/number.
    ";
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).expect("valid regex"));

    let do_try_highlight = |captures: &Captures<'_, str>| match captures.get(1) {
        Some(content) => highlight_inline_code(content.as_str(), engine_state, stack, head),
        // this shouldn't happen
        None => String::new(),
    };
    RE.replace_all(text, do_try_highlight)
}

/// The source text an alias expands to, as written: `str trim` for `alias st = str trim`
fn alias_expansion<'a>(alias: &Alias, engine_state: &'a EngineState) -> Cow<'a, str> {
    String::from_utf8_lossy(engine_state.get_span_contents(alias.wrapped_call.span))
}

#[allow(clippy::too_many_arguments)]
fn get_alias_documentation(
    long_desc: &mut String,
    command: &dyn Command,
    help_style: &HelpStyle,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) {
    let help_section_name = &help_style.section_name;
    let help_subcolor_one = &help_style.subcolor_one;

    let alias_name = command.name();

    write!(
        long_desc,
        "{help_section_name}Alias{RESET}: {help_subcolor_one}{alias_name}{RESET}"
    )
    .expect("writing to a String is infallible");
    long_desc.push_str("\n\n");

    let Some(alias) = command.as_alias() else {
        // this is already checked in `help alias`, but just omit the expansion if this is somehow not actually an alias
        return;
    };

    write!(
        long_desc,
        "{help_section_name}Expansion{RESET}:\n  {}",
        nu_highlight_string(
            &alias_expansion(alias, engine_state),
            engine_state,
            stack,
            head
        )
    )
    .expect("writing to a String is infallible");
}

#[allow(clippy::too_many_arguments)]
fn get_command_documentation(
    long_desc: &mut String,
    command: &dyn Command,
    sig: &Signature,
    nu_config: &Config,
    help_style: &HelpStyle,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) {
    let help_section_name = &help_style.section_name;
    let help_subcolor_one = &help_style.subcolor_one;

    let cmd_name = &sig.name;

    if !sig.search_terms.is_empty() {
        write!(
            long_desc,
            "{help_section_name}Search terms{RESET}: {help_subcolor_one}{}{RESET}\n\n",
            sig.search_terms.join(", "),
        )
        .expect("writing to a String is infallible");
    }

    write!(
        long_desc,
        "{help_section_name}Usage{RESET}:\n  > {}\n",
        sig.call_signature()
    )
    .expect("writing to a String is infallible");

    // TODO: improve the subcommand name resolution
    // issues:
    // - Subcommands are included violating module scoping
    //   - https://github.com/nushell/nushell/issues/11447
    //   - https://github.com/nushell/nushell/issues/11625
    let subcommand_prefix = format!("{cmd_name} ");
    // Every name in scope that lists a declaration as a subcommand, with the name to print and,
    // for an alias of a subcommand, the name of the subcommand it runs. A declaration can be
    // registered under several names (a script's `main bar` is also `script.nu bar`, #17719) and
    // only some of them may match, so every name is checked.
    let mut candidates: Vec<_> = engine_state
        .get_decls_sorted(false)
        .into_iter()
        .filter_map(|(overlay_name, decl_id)| {
            let decl = engine_state.get_decl(decl_id);
            let name = String::from_utf8_lossy(&overlay_name);

            // Prefer the overlay name for display, falling back to the declared name: a module
            // command is registered as `spam foo bar` but documented under its bare name
            // `foo bar`. An alias of a subcommand (`alias st = str trim` under `help str`) is
            // listed under the name it's called by, and `wraps` keeps the subcommand's name so the
            // alias sorts right after it. That name comes from the wrapped declaration, not from
            // the alias's signature, which is a copy taken before a script's `main bar` was
            // renamed to `script.nu bar`.
            let (name_to_print, wraps) = if name.starts_with(&subcommand_prefix) {
                (name.into_owned(), None)
            } else if decl.name().starts_with(&subcommand_prefix) {
                (decl.name().to_owned(), None)
            } else {
                let Expr::Call(call) = &decl.as_alias()?.wrapped_call.expr else {
                    return None;
                };
                let wrapped = engine_state
                    .get_signature(engine_state.get_decl(call.decl_id))
                    .name;
                if !wrapped.starts_with(&subcommand_prefix) {
                    return None;
                }
                (name.into_owned(), Some(wrapped))
            };

            // A hidden declaration can't be called, so it isn't listed. `find_decl` also rejects a
            // name that now resolves to another declaration.
            (engine_state.find_decl(&overlay_name, &[]) == Some(decl_id)).then_some((
                decl_id,
                name_to_print,
                wraps,
            ))
        })
        .collect();
    // A declaration that matches under one of its own names is listed under that name, not as an
    // alias of the subcommand it runs: a script's `alias "main baz" = main bar` is registered as
    // both `main baz` and `script.nu baz`, and is listed as `script.nu baz`. The sort is stable,
    // so within each group the names stay in alphabetical order and the first one wins.
    candidates.sort_by_key(|(_, _, wraps)| wraps.is_some());

    let mut subcommands = vec![];
    // Declarations already added to `subcommands`, so each one is listed once.
    let mut seen = HashSet::new();
    for (decl_id, name_to_print, wraps) in candidates {
        let decl = engine_state.get_decl(decl_id);

        // Don't display removed/deprecated commands in the Subcommands list.
        if !seen.insert(decl_id)
            || matches!(engine_state.get_signature(decl).category, Category::Removed)
        {
            continue;
        }

        let description = match decl.as_alias() {
            // Every alias shows what it runs. A doc comment on the alias replaces its default
            // "Alias for `...`" description, but the comment still shows in the alias's own help.
            // The expansion is joined onto one line to keep one entry per line, and highlighted
            // directly because it may contain backticks of its own.
            Some(alias) => {
                let expansion = alias_expansion(alias, engine_state)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                let expansion = if nu_config.use_ansi_coloring.get(engine_state) {
                    highlight_inline_code(&expansion, engine_state, stack, head)
                } else {
                    format!("`{expansion}`")
                };
                Cow::Owned(format!("Alias for {expansion}"))
            }
            None => highlight_code(decl.description(), engine_state, stack, head),
        };

        // If it's a plugin, alias, or custom command, display that information in the help
        let command_type = decl.command_type();
        let line = if command_type == CommandType::Plugin
            || command_type == CommandType::Alias
            || command_type == CommandType::Custom
        {
            format!(
                "  {help_subcolor_one}{name_to_print} {help_section_name}({command_type}){RESET} - {description}"
            )
        } else {
            format!("  {help_subcolor_one}{name_to_print}{RESET} - {description}")
        };

        // Sorted by name, except that an alias of a subcommand goes right after that subcommand.
        let (sort_name, after_wrapped) = match wraps {
            Some(wrapped) => (wrapped, true),
            None => (name_to_print, false),
        };
        subcommands.push((sort_name, after_wrapped, line));
    }

    if !subcommands.is_empty() {
        write!(long_desc, "\n{help_section_name}Subcommands{RESET}:\n")
            .expect("writing to a String is infallible");
        subcommands.sort();
        // `seen` lists each declaration once, but two declarations can still print the same line,
        // such as a top-level `foo bar` and a module's `foo bar` shown under its bare name.
        subcommands.dedup();
        for (_, _, line) in subcommands {
            long_desc.push_str(&line);
            long_desc.push('\n');
        }
    }

    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(sig, help_style, |v| match v {
            FormatterValue::DefaultValue(value) => nu_highlight_string(
                &value.to_parsable_string(", ", nu_config),
                engine_state,
                stack,
                head,
            ),
            FormatterValue::CodeString(text) => {
                highlight_code(text, engine_state, stack, head).to_string()
            }
        }))
    }

    write!(
        long_desc,
        "\n{help_section_name}Command Type{RESET}:\n  > {}\n",
        command.command_type()
    )
    .expect("writing to a String is infallible");

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        write!(long_desc, "\n{help_section_name}Parameters{RESET}:\n")
            .expect("writing to a String is infallible");
        for positional in &sig.required_positional {
            write_positional(
                long_desc,
                positional,
                PositionalKind::Required,
                help_style,
                nu_config,
                engine_state,
                stack,
                head,
            );
        }
        for positional in &sig.optional_positional {
            write_positional(
                long_desc,
                positional,
                PositionalKind::Optional,
                help_style,
                nu_config,
                engine_state,
                stack,
                head,
            );
        }

        if let Some(rest_positional) = &sig.rest_positional {
            write_positional(
                long_desc,
                rest_positional,
                PositionalKind::Rest,
                help_style,
                nu_config,
                engine_state,
                stack,
                head,
            );
        }
    }

    fn get_term_width() -> usize {
        if let Ok((w, _h)) = terminal_size() {
            w as usize
        } else {
            80
        }
    }

    if !command.is_keyword()
        && !sig.input_output_types.is_empty()
        && let Some(decl_id) = engine_state.find_decl(b"table", &[])
    {
        let mut vals = vec![];
        for (input, output) in &sig.input_output_types {
            vals.push(Value::record(
                record! {
                    "input" => Value::string(input.to_string(), head),
                    "output" => Value::string(output.to_string(), head),
                },
                head,
            ));
        }

        let caller_stack = &mut Stack::new().collect_value();
        if let Ok(result) = eval_call::<WithoutDebug>(
            engine_state,
            caller_stack,
            &Call {
                decl_id,
                head,
                arguments: vec![Argument::Named((
                    Spanned {
                        item: "width".to_string(),
                        span: head,
                    },
                    None,
                    Some(Expression::new_unknown(
                        Expr::Int(get_term_width() as i64 - 2), // padding, see below
                        head,
                        Type::Int,
                    )),
                ))],
                parser_info: HashMap::new(),
            },
            PipelineData::value(Value::list(vals, head), None),
        ) && let Ok((str, ..)) = result.collect_string_strict(head)
        {
            writeln!(long_desc, "\n{help_section_name}Input/output types{RESET}:")
                .expect("writing to a String is infallible");
            for line in str.lines() {
                writeln!(long_desc, "  {line}").expect("writing to a String is infallible");
            }
        }
    }

    let examples = command.examples();

    if !examples.is_empty() {
        write!(long_desc, "\n{help_section_name}Examples{RESET}:")
            .expect("writing to a String is infallible");
    }

    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(&highlight_code(
            example.description,
            engine_state,
            stack,
            head,
        ));

        if !nu_config.use_ansi_coloring.get(engine_state) {
            write!(long_desc, "\n  > {}\n", example.example)
                .expect("writing to a String is infallible");
        } else {
            let code_string = nu_highlight_string(example.example, engine_state, stack, head);
            write!(long_desc, "\n  > {code_string}\n").expect("writing to a String is infallible");
        };

        if let Some(result) = &example.result {
            let mut table_call = Call::new(head);
            if example.example.ends_with("--collapse") {
                // collapse the result
                table_call.add_named((
                    Spanned {
                        item: "collapse".to_string(),
                        span: head,
                    },
                    None,
                    None,
                ))
            } else {
                // expand the result
                table_call.add_named((
                    Spanned {
                        item: "expand".to_string(),
                        span: head,
                    },
                    None,
                    None,
                ))
            }
            table_call.add_named((
                Spanned {
                    item: "width".to_string(),
                    span: head,
                },
                None,
                Some(Expression::new_unknown(
                    Expr::Int(get_term_width() as i64 - 2),
                    head,
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
                            PipelineData::value(result.clone(), None),
                        )
                        .ok()
                });

            for item in table.into_iter().flatten() {
                writeln!(
                    long_desc,
                    "  {}",
                    item.to_expanded_string("", nu_config)
                        .trim_end()
                        .trim_start_matches(|c: char| c.is_whitespace() && c != ' ')
                        .replace('\n', "\n  ")
                )
                .expect("writing to a String is infallible");
            }
        }
    }

    long_desc.push('\n');
}

fn update_ansi_from_config(
    ansi_code: &mut String,
    engine_state: &EngineState,
    nu_config: &Config,
    theme_component: &str,
    head: Span,
) {
    if let Some(color) = &nu_config.color_config.get(theme_component) {
        let caller_stack = &mut Stack::new().collect_value();
        let span_id = UNKNOWN_SPAN_ID;

        let argument_opt = get_argument_for_color_value(nu_config, color, head, span_id);

        // Call ansi command using argument
        if let Some(argument) = argument_opt
            && let Some(decl_id) = engine_state.find_decl(b"ansi", &[])
            && let Ok(result) = eval_call::<WithoutDebug>(
                engine_state,
                caller_stack,
                &Call {
                    decl_id,
                    head,
                    arguments: vec![argument],
                    parser_info: HashMap::new(),
                },
                PipelineData::empty(),
            )
            && let Ok((str, ..)) = result.collect_string_strict(head)
        {
            *ansi_code = str;
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
                span,
                span_id,
                Type::Record(
                    vec![
                        ("fg".to_string(), Type::String),
                        ("attr".to_string(), Type::String),
                    ]
                    .into(),
                ),
            )))
        }
        Value::String { val, .. } => Some(Argument::Positional(Expression::new_existing(
            Expr::String(val.clone()),
            span,
            span_id,
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
    pub fn update_from_config(
        &mut self,
        engine_state: &EngineState,
        nu_config: &Config,
        head: Span,
    ) {
        update_ansi_from_config(
            &mut self.section_name,
            engine_state,
            nu_config,
            "shape_string",
            head,
        );
        update_ansi_from_config(
            &mut self.subcolor_one,
            engine_state,
            nu_config,
            "shape_external",
            head,
        );
        update_ansi_from_config(
            &mut self.subcolor_two,
            engine_state,
            nu_config,
            "shape_block",
            head,
        );
    }
}

#[derive(PartialEq)]
enum PositionalKind {
    Required,
    Optional,
    Rest,
}

#[allow(clippy::too_many_arguments)]
fn write_positional(
    long_desc: &mut String,
    positional: &PositionalArg,
    arg_kind: PositionalKind,
    help_style: &HelpStyle,
    nu_config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
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
            write!(
                long_desc,
                "{help_subcolor_one}\"{}\" + {RESET}<{help_subcolor_two}{}{RESET}>",
                String::from_utf8_lossy(kw),
                shape,
            )
            .expect("writing to a String is infallible");
        }
        _ => {
            write!(
                long_desc,
                "{help_subcolor_one}{}{RESET} <{help_subcolor_two}{}{RESET}>",
                positional.name, positional.shape,
            )
            .expect("writing to a String is infallible");
        }
    };
    if !positional.desc.is_empty() || arg_kind == PositionalKind::Optional {
        write!(
            long_desc,
            ": {}",
            highlight_code(&positional.desc, engine_state, stack, head)
        )
        .expect("writing to a String is infallible");
    }
    if arg_kind == PositionalKind::Optional {
        if let Some(value) = &positional.default_value {
            write!(
                long_desc,
                " (optional, default: {})",
                nu_highlight_string(
                    &value.to_parsable_string(", ", nu_config),
                    engine_state,
                    stack,
                    head
                )
            )
            .expect("writing to a String is infallible");
        } else {
            long_desc.push_str(" (optional)");
        };
    }
    long_desc.push('\n');
}

/// Helper for `get_flags_section`
///
/// The formatter with access to nu-highlight must be passed to `get_flags_section`, but it's not possible
/// to pass separate closures since they both need `&mut Stack`, so this enum lets us differentiate between
/// default values to be formatted and strings which might contain code in backticks to be highlighted.
pub enum FormatterValue<'a> {
    /// Default value to be styled
    DefaultValue(&'a Value),
    /// String which might have code in backticks to be highlighted
    CodeString(&'a str),
}

fn write_flag_to_long_desc<F>(
    flag: &nu_protocol::Flag,
    long_desc: &mut String,
    help_subcolor_one: &str,
    help_subcolor_two: &str,
    formatter: &mut F,
) where
    F: FnMut(FormatterValue) -> String,
{
    // Indentation
    long_desc.push_str("  ");
    // Short flag shown before long flag
    if let Some(short) = flag.short {
        write!(long_desc, "{help_subcolor_one}-{short}{RESET}")
            .expect("writing to a String is infallible");
        if flag.long_name().is_some() {
            write!(long_desc, "{DEFAULT_COLOR},{RESET} ")
                .expect("writing to a String is infallible");
        }
    }
    if let Some(long) = flag.long_name() {
        write!(long_desc, "{help_subcolor_one}--{long}{RESET}")
            .expect("writing to a String is infallible");
    }
    if flag.required {
        long_desc.push_str(" (required parameter)")
    }
    // Type/Syntax shape info
    if let Some(arg) = &flag.arg {
        write!(long_desc, " <{help_subcolor_two}{arg}{RESET}>")
            .expect("writing to a String is infallible");
    }
    if !flag.desc.is_empty() {
        write!(
            long_desc,
            ": {}",
            formatter(FormatterValue::CodeString(&flag.desc))
        )
        .expect("writing to a String is infallible");
    }
    if let Some(value) = &flag.default_value {
        write!(
            long_desc,
            " (default: {})",
            formatter(FormatterValue::DefaultValue(value))
        )
        .expect("writing to a String is infallible");
    }
    long_desc.push('\n');
}

pub fn get_flags_section<F>(
    signature: &Signature,
    help_style: &HelpStyle,
    mut formatter: F, // format default Value or text with code (because some calls cannot access config or nu-highlight)
) -> String
where
    F: FnMut(FormatterValue) -> String,
{
    let help_section_name = &help_style.section_name;
    let help_subcolor_one = &help_style.subcolor_one;
    let help_subcolor_two = &help_style.subcolor_two;

    let mut long_desc = String::new();
    write!(long_desc, "\n{help_section_name}Flags{RESET}:\n")
        .expect("writing to a String is infallible");

    let help = signature.named.iter().find(|flag| flag.long == "help");
    let required = signature.named.iter().filter(|flag| flag.required);
    let optional = signature
        .named
        .iter()
        .filter(|flag| !flag.required && flag.long != "help");

    let flags = required.chain(help).chain(optional);

    for flag in flags {
        write_flag_to_long_desc(
            flag,
            &mut long_desc,
            help_subcolor_one,
            help_subcolor_two,
            &mut formatter,
        );
    }

    long_desc
}

#[cfg(test)]
mod tests {
    use nu_protocol::UseAnsiColoring;

    use super::*;

    #[test]
    fn test_code_formatting() {
        let mut engine_state = EngineState::new();
        let mut stack = Stack::new();

        // force coloring on for test
        let mut config = engine_state.get_config().as_ref().clone();
        config.use_ansi_coloring = UseAnsiColoring::True;
        engine_state.set_config(config);

        // using Cow::Owned here to mean a match, since the content changed,
        // and borrowed to mean not a match, since the content didn't change

        // match: typical example
        let haystack = "Run the `foo` command";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Owned(_)
        ));

        // no match: backticks preceded by alphanum
        let haystack = "foo`bar`";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Borrowed(_)
        ));

        // match: command at beginning of string is ok
        let haystack = "`my-command` is cool";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Owned(_)
        ));

        // match: preceded and followed by newline is ok
        let haystack = "
        `command`
        ";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Owned(_)
        ));

        // no match: newline between backticks
        let haystack = "// hello `beautiful \n world`";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Borrowed(_)
        ));

        // match: backticks followed by period, not letter/number
        let haystack = "try running `my cool command`.";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Owned(_)
        ));

        // match: backticks enclosed by parenthesis, not letter/number
        let haystack = "a command (`my cool command`).";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Owned(_)
        ));

        // no match: only characters inside backticks are backticks
        // (the regex sees two backtick pairs with a single backtick inside, which doesn't qualify)
        let haystack = "```\ncode block\n```";
        assert!(matches!(
            highlight_code(haystack, &engine_state, &mut stack, Span::test_data()),
            Cow::Borrowed(_)
        ));
    }
}
