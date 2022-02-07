<<<<<<< HEAD
use crate::evaluate::scope::Scope;
use crate::whole_stream_command::WholeStreamCommand;
use indexmap::IndexMap;
use itertools::Itertools;
use nu_protocol::{NamedType, PositionalType, Signature, UntaggedValue, Value};
use nu_source::PrettyDebug;
=======
use itertools::Itertools;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    Example, IntoPipelineData, Signature, Span, Value,
};
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
use std::collections::HashMap;

const COMMANDS_DOCS_DIR: &str = "docs/commands";

#[derive(Default)]
pub struct DocumentationConfig {
    no_subcommands: bool,
<<<<<<< HEAD
=======
    //FIXME: add back in color support
    #[allow(dead_code)]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    no_color: bool,
    brief: bool,
}

<<<<<<< HEAD
fn generate_doc(name: &str, scope: &Scope) -> IndexMap<String, Value> {
    let mut row_entries = IndexMap::new();
    let command = scope
        .get_command(name)
        .unwrap_or_else(|| panic!("Expected command '{}' from names to be in registry", name));
    row_entries.insert(
        "name".to_owned(),
        UntaggedValue::string(name).into_untagged_value(),
    );
    row_entries.insert(
        "usage".to_owned(),
        UntaggedValue::string(command.usage()).into_untagged_value(),
    );
    retrieve_doc_link(name).and_then(|link| {
        row_entries.insert(
            "doc_link".to_owned(),
            UntaggedValue::string(link).into_untagged_value(),
        )
    });
    row_entries.insert(
        "documentation".to_owned(),
        UntaggedValue::string(get_documentation(
            command.stream_command(),
            scope,
=======
fn generate_doc(
    name: &str,
    engine_state: &EngineState,
    stack: &mut Stack,
    head: Span,
) -> (Vec<String>, Vec<Value>) {
    let mut cols = vec![];
    let mut vals = vec![];

    let command = engine_state
        .find_decl(name.as_bytes())
        .map(|decl_id| engine_state.get_decl(decl_id))
        .unwrap_or_else(|| panic!("Expected command '{}' from names to be in registry", name));

    cols.push("name".to_string());
    vals.push(Value::String {
        val: name.into(),
        span: head,
    });

    cols.push("usage".to_string());
    vals.push(Value::String {
        val: command.usage().to_owned(),
        span: head,
    });

    if let Some(link) = retrieve_doc_link(name) {
        cols.push("doc_link".into());
        vals.push(Value::String {
            val: link,
            span: head,
        });
    }

    cols.push("documentation".to_owned());
    vals.push(Value::String {
        val: get_documentation(
            &command.signature(),
            &command.examples(),
            engine_state,
            stack,
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            &DocumentationConfig {
                no_subcommands: true,
                no_color: true,
                brief: false,
            },
<<<<<<< HEAD
        ))
        .into_untagged_value(),
    );
    row_entries
}

// generate_docs gets the documentation from each command and returns a Table as output
pub fn generate_docs(scope: &Scope) -> Value {
    let mut sorted_names = scope.get_command_names();
    sorted_names.sort();

    // cmap will map parent commands to it's subcommands e.g. to -> [to csv, to yaml, to bson]
    let mut cmap: HashMap<String, Vec<String>> = HashMap::new();
    for name in &sorted_names {
        if name.contains(' ') {
            let split_name = name.split_whitespace().collect_vec();
            let parent_name = split_name.first().expect("Expected a parent command name");
            if cmap.contains_key(*parent_name) {
                let sub_names = cmap
                    .get_mut(*parent_name)
                    .expect("Expected an entry for parent");
                sub_names.push(name.to_owned());
            }
        } else {
            cmap.insert(name.to_owned(), Vec::new());
=======
        ),
        span: head,
    });

    (cols, vals)
}

// generate_docs gets the documentation from each command and returns a Table as output
pub fn generate_docs(engine_state: &EngineState, stack: &mut Stack, head: Span) -> Value {
    let signatures = engine_state.get_signatures(true);

    // cmap will map parent commands to it's subcommands e.g. to -> [to csv, to yaml, to bson]
    let mut cmap: HashMap<String, Vec<String>> = HashMap::new();
    for sig in &signatures {
        if sig.name.contains(' ') {
            let mut split_name = sig.name.split_whitespace();
            let parent_name = split_name.next().expect("Expected a parent command name");
            if cmap.contains_key(parent_name) {
                let sub_names = cmap
                    .get_mut(parent_name)
                    .expect("Expected an entry for parent");
                sub_names.push(sig.name.to_owned());
            }
        } else {
            cmap.insert(sig.name.to_owned(), Vec::new());
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        };
    }
    // Return documentation for each command
    // Sub-commands are nested under their respective parent commands
    let mut table = Vec::new();
<<<<<<< HEAD
    for name in &sorted_names {
        // Must be a sub-command, skip since it's being handled underneath when we hit the parent command
        if !cmap.contains_key(name) {
            continue;
        }
        let mut row_entries = generate_doc(name, scope);
        // Iterate over all the subcommands of the parent command
        let mut sub_table = Vec::new();
        for sub_name in cmap.get(name).unwrap_or(&Vec::new()) {
            let sub_row = generate_doc(sub_name, scope);
            sub_table.push(UntaggedValue::row(sub_row).into_untagged_value());
        }

        if !sub_table.is_empty() {
            row_entries.insert(
                "subcommands".to_owned(),
                UntaggedValue::table(&sub_table).into_untagged_value(),
            );
        }
        table.push(UntaggedValue::row(row_entries).into_untagged_value());
    }
    UntaggedValue::table(&table).into_untagged_value()
=======
    for sig in &signatures {
        // Must be a sub-command, skip since it's being handled underneath when we hit the parent command
        if !cmap.contains_key(&sig.name) {
            continue;
        }
        let mut row_entries = generate_doc(&sig.name, engine_state, stack, head);
        // Iterate over all the subcommands of the parent command
        let mut sub_table = Vec::new();
        for sub_name in cmap.get(&sig.name).unwrap_or(&Vec::new()) {
            let (cols, vals) = generate_doc(sub_name, engine_state, stack, head);
            sub_table.push(Value::Record {
                cols,
                vals,
                span: head,
            });
        }

        if !sub_table.is_empty() {
            row_entries.0.push("subcommands".into());
            row_entries.1.push(Value::List {
                vals: sub_table,
                span: head,
            });
        }
        table.push(Value::Record {
            cols: row_entries.0,
            vals: row_entries.1,
            span: head,
        });
    }
    Value::List {
        vals: table,
        span: head,
    }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

fn retrieve_doc_link(name: &str) -> Option<String> {
    let doc_name = name.split_whitespace().join("_"); // Because .replace(" ", "_") didn't work
    let mut entries =
        std::fs::read_dir(COMMANDS_DOCS_DIR).expect("Directory for command docs are missing!");
    entries.find_map(|r| {
        r.map_or(None, |de| {
            if de.file_name().to_string_lossy() == format!("{}.{}", &doc_name, "md") {
                Some(format!("/commands/{}.{}", &doc_name, "html"))
            } else {
                None
            }
        })
    })
}

#[allow(clippy::cognitive_complexity)]
pub fn get_documentation(
<<<<<<< HEAD
    cmd: &dyn WholeStreamCommand,
    scope: &Scope,
    config: &DocumentationConfig,
) -> String {
    let cmd_name = cmd.name();
    let signature = cmd.signature();
    let mut long_desc = String::new();

    let usage = &cmd.usage();
=======
    sig: &Signature,
    examples: &[Example],
    engine_state: &EngineState,
    stack: &mut Stack,
    config: &DocumentationConfig,
) -> String {
    let cmd_name = &sig.name;
    let mut long_desc = String::new();

    let usage = &sig.usage;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    if !usage.is_empty() {
        long_desc.push_str(usage);
        long_desc.push_str("\n\n");
    }

<<<<<<< HEAD
    let extra_usage = if config.brief { "" } else { &cmd.extra_usage() };
=======
    let extra_usage = if config.brief { "" } else { &sig.extra_usage };
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    if !extra_usage.is_empty() {
        long_desc.push_str(extra_usage);
        long_desc.push_str("\n\n");
    }

    let mut subcommands = vec![];
    if !config.no_subcommands {
<<<<<<< HEAD
        for name in scope.get_command_names() {
            if name.starts_with(&format!("{} ", cmd_name)) {
                let subcommand = scope.get_command(&name).expect("This shouldn't happen");

                subcommands.push(format!("  {} - {}", name, subcommand.usage()));
=======
        let signatures = engine_state.get_signatures(true);
        for sig in signatures {
            if sig.name.starts_with(&format!("{} ", cmd_name)) {
                subcommands.push(format!("  {} - {}", sig.name, sig.usage));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            }
        }
    }

<<<<<<< HEAD
    let mut one_liner = String::new();
    one_liner.push_str(&signature.name);
    one_liner.push(' ');

    for positional in &signature.positional {
        match &positional.0 {
            PositionalType::Mandatory(name, _m) => {
                one_liner.push_str(&format!("<{}> ", name));
            }
            PositionalType::Optional(name, _o) => {
                one_liner.push_str(&format!("({}) ", name));
            }
        }
    }

    if signature.rest_positional.is_some() {
        one_liner.push_str("...args ");
    }

    if !subcommands.is_empty() {
        one_liner.push_str("<subcommand> ");
    }

    if !signature.named.is_empty() {
        one_liner.push_str("{flags} ");
    }

    long_desc.push_str(&format!("Usage:\n  > {}\n", one_liner));
=======
    long_desc.push_str(&format!("Usage:\n  > {}\n", sig.call_signature()));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

    if !subcommands.is_empty() {
        long_desc.push_str("\nSubcommands:\n");
        subcommands.sort();
        long_desc.push_str(&subcommands.join("\n"));
        long_desc.push('\n');
    }

<<<<<<< HEAD
    if !signature.positional.is_empty() || signature.rest_positional.is_some() {
        long_desc.push_str("\nParameters:\n");
        for positional in &signature.positional {
            match &positional.0 {
                PositionalType::Mandatory(name, _m) => {
                    long_desc.push_str(&format!("  <{}> {}\n", name, positional.1));
                }
                PositionalType::Optional(name, _o) => {
                    long_desc.push_str(&format!("  ({}) {}\n", name, positional.1));
                }
            }
        }

        if let Some(rest_positional) = &signature.rest_positional {
            long_desc.push_str(&format!("  ...args: {}\n", rest_positional.2));
        }
    }
    if !signature.named.is_empty() {
        long_desc.push_str(&get_flags_section(&signature))
    }

    let palette = crate::shell::palette::DefaultPalette {};
    let examples = cmd.examples();
    if !examples.is_empty() {
        long_desc.push_str("\nExamples:");
    }
=======
    if !sig.named.is_empty() {
        long_desc.push_str(&get_flags_section(sig))
    }

    if !sig.required_positional.is_empty()
        || !sig.optional_positional.is_empty()
        || sig.rest_positional.is_some()
    {
        long_desc.push_str("\nParameters:\n");
        for positional in &sig.required_positional {
            long_desc.push_str(&format!("  {}: {}\n", positional.name, positional.desc));
        }
        for positional in &sig.optional_positional {
            long_desc.push_str(&format!(
                "  (optional) {}: {}\n",
                positional.name, positional.desc
            ));
        }

        if let Some(rest_positional) = &sig.rest_positional {
            long_desc.push_str(&format!(
                "  ...{}: {}\n",
                rest_positional.name, rest_positional.desc
            ));
        }
    }

    if !examples.is_empty() {
        long_desc.push_str("\nExamples:");
    }

>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_color {
            long_desc.push_str(&format!("\n  > {}\n", example.example));
<<<<<<< HEAD
        } else {
            let colored_example =
                crate::shell::painter::Painter::paint_string(example.example, scope, &palette);
            long_desc.push_str(&format!("\n  > {}\n", colored_example));
=======
        } else if let Some(highlighter) = engine_state.find_decl(b"nu-highlight") {
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        }
    }

    long_desc.push('\n');

    long_desc
}

<<<<<<< HEAD
fn get_flags_section(signature: &Signature) -> String {
    let mut long_desc = String::new();
    long_desc.push_str("\nFlags:\n");
    for (flag, ty) in &signature.named {
        let msg = match ty.0 {
            NamedType::Switch(s) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{}{} {}\n",
                        c,
                        flag,
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{}{} {}\n",
                        flag,
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
            NamedType::Mandatory(s, m) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{} <{}> (required parameter){} {}\n",
                        c,
                        flag,
                        m.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{} <{}> (required parameter){} {}\n",
                        flag,
                        m.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
            NamedType::Optional(s, o) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{} <{}>{} {}\n",
                        c,
                        flag,
                        o.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{} <{}>{} {}\n",
                        flag,
                        o.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
=======
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
                format!("  --{} {:?}\n      {}\n", flag.long, arg, flag.desc)
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        };
        long_desc.push_str(&msg);
    }
    long_desc
}

<<<<<<< HEAD
pub fn get_brief_help(cmd: &dyn WholeStreamCommand, scope: &Scope) -> String {
    get_documentation(
        cmd,
        scope,
=======
pub fn get_brief_help(
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        &DocumentationConfig {
            no_subcommands: false,
            no_color: false,
            brief: true,
        },
    )
}

<<<<<<< HEAD
pub fn get_full_help(cmd: &dyn WholeStreamCommand, scope: &Scope) -> String {
    get_documentation(cmd, scope, &DocumentationConfig::default())
=======
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
