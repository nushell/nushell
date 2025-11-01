use miette::IntoDiagnostic;
use nu_cli::NuCompleter;
use nu_parser::{FlatShape, flatten_block, parse};
use nu_protocol::{
    DeclId, ShellError, Span, Value, VarId,
    engine::{EngineState, Stack, StateWorkingSet},
    report_shell_error,
    shell_error::io::{IoError, IoErrorExt, NotFound},
};
use reedline::Completer;
use serde_json::{Value as JsonValue, json};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
enum Id {
    Variable(VarId),
    Declaration(DeclId),
    Value(FlatShape),
}

fn find_id(
    working_set: &mut StateWorkingSet,
    file_path: &str,
    file: &[u8],
    location: &Value,
) -> Option<(Id, usize, Span)> {
    let file_id = working_set.add_file(file_path.to_string(), file);
    let offset = working_set.get_span_for_file(file_id).start;
    let _ = working_set.files.push(file_path.into(), Span::unknown());
    let block = parse(working_set, Some(file_path), file, false);
    let flattened = flatten_block(working_set, &block);

    if let Ok(location) = location.as_int() {
        let location = location as usize + offset;
        for item in flattened {
            if location >= item.0.start && location < item.0.end {
                match &item.1 {
                    FlatShape::Variable(var_id) | FlatShape::VarDecl(var_id) => {
                        return Some((Id::Variable(*var_id), offset, item.0));
                    }
                    FlatShape::InternalCall(decl_id) => {
                        return Some((Id::Declaration(*decl_id), offset, item.0));
                    }
                    _ => return Some((Id::Value(item.1), offset, item.0)),
                }
            }
        }
        None
    } else {
        None
    }
}

fn read_in_file<'a>(
    engine_state: &'a mut EngineState,
    file_path: &str,
) -> (Vec<u8>, StateWorkingSet<'a>) {
    let file = std::fs::read(file_path)
        .map_err(|err| {
            ShellError::Io(IoError::new_with_additional_context(
                err.not_found_as(NotFound::File),
                Span::unknown(),
                PathBuf::from(file_path),
                "Could not read file",
            ))
        })
        .unwrap_or_else(|err| {
            report_shell_error(engine_state, &err);
            std::process::exit(1);
        });

    engine_state.file = Some(PathBuf::from(file_path));

    let working_set = StateWorkingSet::new(engine_state);

    (file, working_set)
}

pub fn check(engine_state: &mut EngineState, file_path: &str, max_errors: &Value) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
    engine_state.generate_nu_constant();

    let mut working_set = StateWorkingSet::new(engine_state);
    let file = std::fs::read(file_path);

    let max_errors = if let Ok(max_errors) = max_errors.as_int() {
        max_errors as usize
    } else {
        100
    };

    if let Ok(contents) = file {
        let offset = working_set.next_span_start();
        let _ = working_set.files.push(file_path.into(), Span::unknown());
        let block = parse(&mut working_set, Some(file_path), &contents, false);

        for (idx, err) in working_set.parse_errors.iter().enumerate() {
            if idx >= max_errors {
                // eprintln!("Too many errors, stopping here. idx: {idx} max_errors: {max_errors}");
                break;
            }
            let mut span = err.span();
            span.start -= offset;
            span.end -= offset;

            let msg = err.to_string();

            println!(
                "{}",
                json!({
                    "type": "diagnostic",
                    "severity": "Error",
                    "message": msg,
                    "span": {
                        "start": span.start,
                        "end": span.end
                    }
                })
            );
        }

        let flattened = flatten_block(&working_set, &block);

        for flat in flattened {
            if let FlatShape::VarDecl(var_id) = flat.1 {
                let var = working_set.get_variable(var_id);
                println!(
                    "{}",
                    json!({
                        "type": "hint",
                        "typename": var.ty.to_string(),
                        "position": {
                            "start": flat.0.start - offset,
                            "end": flat.0.end - offset
                        }
                    })
                );
            }
        }
    }
}

pub fn goto_def(engine_state: &mut EngineState, file_path: &str, location: &Value) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::Declaration(decl_id), ..)) => {
            let result = working_set.get_decl(decl_id);
            if let Some(block_id) = result.block_id() {
                let block = working_set.get_block(block_id);
                if let Some(span) = &block.span {
                    for file in working_set.files() {
                        if file.covered_span.contains(span.start) {
                            println!(
                                "{}",
                                json!(
                                    {
                                        "file": &*file.name,
                                        "start": span.start - file.covered_span.start,
                                        "end": span.end - file.covered_span.start,
                                    }
                                )
                            );
                            return;
                        }
                    }
                }
            }
        }
        Some((Id::Variable(var_id), ..)) => {
            let var = working_set.get_variable(var_id);
            for file in working_set.files() {
                if file.covered_span.contains(var.declaration_span.start) {
                    println!(
                        "{}",
                        json!(
                            {
                                "file": &*file.name,
                                "start": var.declaration_span.start - file.covered_span.start,
                                "end": var.declaration_span.end - file.covered_span.start,
                            }
                        )
                    );
                    return;
                }
            }
        }
        _ => {}
    }

    println!("{{}}");
}

pub fn hover(engine_state: &mut EngineState, file_path: &str, location: &Value) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::Declaration(decl_id), offset, span)) => {
            let decl = working_set.get_decl(decl_id);

            let mut description = String::new();

            // first description
            description.push_str(&format!("{}\n", decl.description()));

            // additional description
            if !decl.extra_description().is_empty() {
                description.push_str(&format!("\n{}\n", decl.extra_description()));
            }

            // Usage
            description.push_str("### Usage\n```\n");
            let signature = decl.signature();
            description.push_str(&format!("  {}", signature.name));
            if !signature.named.is_empty() {
                description.push_str(" {flags}")
            }
            for required_arg in &signature.required_positional {
                description.push_str(&format!(" <{}>", required_arg.name));
            }
            for optional_arg in &signature.optional_positional {
                description.push_str(&format!(" <{}?>", optional_arg.name));
            }
            if let Some(arg) = &signature.rest_positional {
                description.push_str(&format!(" <...{}>", arg.name));
            }

            description.push_str("\n```\n");

            // Flags
            if !signature.named.is_empty() {
                description.push_str("\n### Flags\n\n");

                let mut first = true;
                for named in &signature.named {
                    if !first {
                        description.push_str("\\\n");
                    } else {
                        first = false;
                    }
                    description.push_str("  ");
                    if let Some(short_flag) = &named.short {
                        description.push_str(&format!("`-{short_flag}`"));
                    }

                    if !named.long.is_empty() {
                        if named.short.is_some() {
                            description.push_str(", ")
                        }
                        description.push_str(&format!("`--{}`", named.long));
                    }

                    if let Some(arg) = &named.arg {
                        description.push_str(&format!(" `<{}>`", arg.to_type()))
                    }

                    if !named.desc.is_empty() {
                        description.push_str(&format!(" - {}", named.desc));
                    }
                }
                description.push('\n');
            }

            // Parameters
            if !signature.required_positional.is_empty()
                || !signature.optional_positional.is_empty()
                || signature.rest_positional.is_some()
            {
                description.push_str("\n### Parameters\n\n");
                let mut first = true;
                for required_arg in &signature.required_positional {
                    if !first {
                        description.push_str("\\\n");
                    } else {
                        first = false;
                    }

                    description.push_str(&format!(
                        "  `{}: {}`",
                        required_arg.name,
                        required_arg.shape.to_type()
                    ));
                    if !required_arg.desc.is_empty() {
                        description.push_str(&format!(" - {}", required_arg.desc));
                    }
                    description.push('\n');
                }
                for optional_arg in &signature.optional_positional {
                    if !first {
                        description.push_str("\\\n");
                    } else {
                        first = false;
                    }

                    description.push_str(&format!(
                        "  `{}: {}`",
                        optional_arg.name,
                        optional_arg.shape.to_type()
                    ));
                    if !optional_arg.desc.is_empty() {
                        description.push_str(&format!(" - {}", optional_arg.desc));
                    }
                    description.push('\n');
                }
                if let Some(arg) = &signature.rest_positional {
                    if !first {
                        description.push_str("\\\n");
                    }

                    description.push_str(&format!(" `...{}: {}`", arg.name, arg.shape.to_type()));
                    if !arg.desc.is_empty() {
                        description.push_str(&format!(" - {}", arg.desc));
                    }
                    description.push('\n');
                }

                description.push('\n');
            }

            // Input/output types
            if !signature.input_output_types.is_empty() {
                description.push_str("\n### Input/output types\n");

                description.push_str("\n```\n");
                for input_output in &signature.input_output_types {
                    description.push_str(&format!("  {} | {}\n", input_output.0, input_output.1));
                }
                description.push_str("\n```\n");
            }

            // Examples
            if !decl.examples().is_empty() {
                description.push_str("### Example(s)\n```\n");

                for example in decl.examples() {
                    description.push_str(&format!(
                        "```\n  {}\n```\n  {}\n\n",
                        example.description, example.example
                    ));
                }
            }

            println!(
                "{}",
                json!({
                    "hover": description,
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            );
        }
        Some((Id::Variable(var_id), offset, span)) => {
            let var = working_set.get_variable(var_id);

            println!(
                "{}",
                json!({
                    "hover": format!("{}{}", if var.mutable { "mutable " } else { "" }, var.ty),
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            );
        }
        Some((Id::Value(shape), offset, span)) => match shape {
            FlatShape::Binary => println!(
                "{}",
                json!({
                    "hover": "binary",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Bool => println!(
                "{}",
                json!({
                    "hover": "bool",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::DateTime => println!(
                "{}",
                json!({
                    "hover": "datetime",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::External(alias_span) => println!(
                "{}",
                json!({
                    "hover": "external",
                    "span": {
                        "start": alias_span.start - offset,
                        "end": alias_span.end - offset
                    }
                })
            ),
            FlatShape::ExternalArg => println!(
                "{}",
                json!({
                    "hover": "external arg",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Flag => println!(
                "{}",
                json!({
                    "hover": "flag",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Block => println!(
                "{}",
                json!({
                    "hover": "block",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Directory => println!(
                "{}",
                json!({
                    "hover": "directory",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Filepath => println!(
                "{}",
                json!({
                    "hover": "file path",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Float => println!(
                "{}",
                json!({
                    "hover": "float",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::GlobPattern => println!(
                "{}",
                json!({
                    "hover": "glob pattern",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Int => println!(
                "{}",
                json!({
                    "hover": "int",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Keyword => println!(
                "{}",
                json!({
                    "hover": "keyword",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::List => println!(
                "{}",
                json!({
                    "hover": "list",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::MatchPattern => println!(
                "{}",
                json!({
                    "hover": "match-pattern",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Nothing => println!(
                "{}",
                json!({
                    "hover": "nothing",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Range => println!(
                "{}",
                json!({
                    "hover": "range",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Record => println!(
                "{}",
                json!({
                    "hover": "record",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::String => println!(
                "{}",
                json!({
                    "hover": "string",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::RawString => println!(
                "{}",
                json!({
                    "hover": "raw-string",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::StringInterpolation => println!(
                "{}",
                json!({
                    "hover": "string interpolation",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            FlatShape::Table => println!(
                "{}",
                json!({
                    "hover": "table",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
                    }
                })
            ),
            _ => {}
        },
        _ => {}
    }
}

pub fn complete(engine_reference: Arc<EngineState>, file_path: &str, location: &Value) {
    let mut completer = NuCompleter::new(engine_reference, Arc::new(Stack::new()));

    let file = std::fs::read(file_path)
        .into_diagnostic()
        .unwrap_or_else(|_| {
            std::process::exit(1);
        });

    if let Ok(location) = location.as_int() {
        let results = completer.complete(
            &String::from_utf8_lossy(&file)[..location as usize],
            location as usize,
        );
        print!("{{\"completions\": [");
        let mut first = true;
        for result in results {
            if !first {
                print!(", ")
            } else {
                first = false;
            }
            print!("\"{}\"", result.value,)
        }
        println!("]}}");
    }
}

pub fn ast(engine_state: &mut EngineState, file_path: &str) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

    let mut working_set = StateWorkingSet::new(engine_state);
    let file = std::fs::read(file_path);

    if let Ok(contents) = file {
        let offset = working_set.next_span_start();
        let _ = working_set.files.push(file_path.into(), Span::unknown());
        let parsed_block = parse(&mut working_set, Some(file_path), &contents, false);

        let flat = flatten_block(&working_set, &parsed_block);
        let mut json_val: JsonValue = json!([]);
        for (span, shape) in flat {
            let content = String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();

            let json = json!(
                {
                    "type": "ast",
                    "span": {
                        "start": span.start.checked_sub(offset),
                        "end": span.end.checked_sub(offset),
                    },
                    "shape": shape.to_string(),
                    "content": content // may not be necessary, but helpful for debugging
                }
            );
            json_merge(&mut json_val, &json);
        }
        if let Ok(json_str) = serde_json::to_string(&json_val) {
            println!("{json_str}");
        } else {
            println!("{{}}");
        };
    }
}

fn json_merge(a: &mut JsonValue, b: &JsonValue) {
    match (a, b) {
        (JsonValue::Object(a), JsonValue::Object(b)) => {
            for (k, v) in b {
                json_merge(a.entry(k).or_insert(JsonValue::Null), v);
            }
        }
        (JsonValue::Array(a), JsonValue::Array(b)) => {
            a.extend(b.clone());
        }
        (JsonValue::Array(a), JsonValue::Object(b)) => {
            a.extend([JsonValue::Object(b.clone())]);
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}
