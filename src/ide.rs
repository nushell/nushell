use miette::IntoDiagnostic;
use nu_cli::{report_error, NuCompleter};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    DeclId, ShellError, Span, Value, VarId,
};
use reedline::Completer;
use serde_json::json;
use std::sync::Arc;

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
    let offset = working_set.next_span_start();
    let (block, _) = parse(working_set, Some(file_path), file, false, &[]);

    let flattened = flatten_block(working_set, &block);

    if let Ok(location) = location.as_i64() {
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
    file_path: &String,
) -> (Vec<u8>, StateWorkingSet<'a>) {
    let file = std::fs::read(file_path)
        .into_diagnostic()
        .unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(
                &working_set,
                &ShellError::FileNotFoundCustom(
                    format!("Could not read file '{}': {:?}", file_path, e.to_string()),
                    Span::unknown(),
                ),
            );
            std::process::exit(1);
        });

    engine_state.start_in_file(Some(file_path));

    let working_set = StateWorkingSet::new(engine_state);

    (file, working_set)
}

pub fn check(engine_state: &mut EngineState, file_path: &String) {
    let mut working_set = StateWorkingSet::new(engine_state);
    let file = std::fs::read(file_path);

    if let Ok(contents) = file {
        let offset = working_set.next_span_start();
        let (block, err) = parse(&mut working_set, Some(file_path), &contents, false, &[]);

        if let Some(err) = err {
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
                        "typename": var.ty,
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

pub fn goto_def(engine_state: &mut EngineState, file_path: &String, location: &Value) {
    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::Declaration(decl_id), offset, _)) => {
            let result = working_set.get_decl(decl_id);
            if let Some(block_id) = result.get_block_id() {
                let block = working_set.get_block(block_id);
                if let Some(span) = &block.span {
                    for file in working_set.files() {
                        if span.start >= file.1 && span.start < file.2 {
                            println!(
                                "{}",
                                json!(
                                    {
                                        "file": file.0,
                                        "start": span.start - offset,
                                        "end": span.end - offset
                                    }
                                )
                            );
                            return;
                        }
                    }
                }
            }
        }
        Some((Id::Variable(var_id), offset, _)) => {
            let var = working_set.get_variable(var_id);
            for file in working_set.files() {
                if var.declaration_span.start >= file.1 && var.declaration_span.start < file.2 {
                    println!(
                        "{}",
                        json!(
                            {
                                "file": file.0,
                                "start": var.declaration_span.start - offset,
                                "end": var.declaration_span.end - offset
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

pub fn hover(engine_state: &mut EngineState, file_path: &String, location: &Value) {
    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::Declaration(decl_id), offset, span)) => {
            let decl = working_set.get_decl(decl_id);

            let mut description = format!("```\n### Signature\n```\n{}\n\n", decl.signature());

            description.push_str(&format!("```\n### Usage\n  {}\n```\n", decl.usage()));

            if !decl.extra_usage().is_empty() {
                description.push_str(&format!(
                    "\n```\n### Extra usage:\n  {}\n```\n",
                    decl.extra_usage()
                ));
            }

            if !decl.examples().is_empty() {
                description.push_str("\n```\n### Example(s)\n```\n");

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
            FlatShape::External => println!(
                "{}",
                json!({
                    "hover": "external",
                    "span": {
                        "start": span.start - offset,
                        "end": span.end - offset
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
                    "hover": "match pattern",
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

pub fn complete(engine_reference: Arc<EngineState>, file_path: &String, location: &Value) {
    let stack = Stack::new();
    let mut completer = NuCompleter::new(engine_reference, stack);

    let file = std::fs::read(file_path)
        .into_diagnostic()
        .unwrap_or_else(|_| {
            std::process::exit(1);
        });

    if let Ok(location) = location.as_i64() {
        let results = completer.complete(&String::from_utf8_lossy(&file), location as usize);
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
