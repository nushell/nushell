use itertools::Itertools;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    Example, IntoPipelineData, Signature, Span, Value,
};
use std::collections::HashMap;

const COMMANDS_DOCS_DIR: &str = "docs/commands";

#[derive(Default)]
pub struct DocumentationConfig {
    no_subcommands: bool,
    //FIXME: add back in color support
    #[allow(dead_code)]
    no_color: bool,
    brief: bool,
}

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
            &DocumentationConfig {
                no_subcommands: true,
                no_color: true,
                brief: false,
            },
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
        };
    }
    // Return documentation for each command
    // Sub-commands are nested under their respective parent commands
    let mut table = Vec::new();
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

    for example in examples {
        long_desc.push('\n');
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_color {
            long_desc.push_str(&format!("\n  > {}\n", example.example));
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
        &DocumentationConfig {
            no_subcommands: false,
            no_color: false,
            brief: true,
        },
    )
}

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
