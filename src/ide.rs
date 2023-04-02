use std::sync::Arc;

use miette::IntoDiagnostic;
use nu_cli::{report_error, NuCompleter};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    DeclId, ShellError, Span, Value, VarId,
};
use reedline::Completer;

enum Id {
    VarId(VarId),
    DeclId(DeclId),
}

fn find_id(
    working_set: &mut StateWorkingSet,
    file_path: &str,
    file: &[u8],
    location: &Value,
) -> Option<(Id, usize)> {
    let offset = working_set.next_span_start();
    let (block, _) = parse(working_set, Some(file_path), file, false, &[]);

    let flattened = flatten_block(working_set, &block);

    if let Ok(location) = location.as_i64() {
        let location = location as usize + offset;
        for item in flattened {
            if location >= item.0.start && location < item.0.end {
                match &item.1 {
                    FlatShape::Variable(var_id) => {
                        return Some((Id::VarId(*var_id), offset));
                    }
                    FlatShape::InternalCall(decl_id) => {
                        return Some((Id::DeclId(*decl_id), offset));
                    }
                    _ => {
                        break;
                    }
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

pub fn goto_def(engine_state: &mut EngineState, file_path: &String, location: &Value) {
    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::DeclId(decl_id), offset)) => {
            let result = working_set.get_decl(decl_id);
            if let Some(block_id) = result.get_block_id() {
                let block = working_set.get_block(block_id);
                if let Some(span) = &block.span {
                    for file in working_set.files() {
                        if span.start >= file.1 && span.start < file.2 {
                            println!(
                                "{{\"file\": \"{}\", \"start\": {}, \"end\": {}}}",
                                file.0,
                                span.start - offset,
                                span.end - offset
                            );
                            return;
                        }
                    }
                }
            }
        }
        Some((Id::VarId(var_id), offset)) => {
            let var = working_set.get_variable(var_id);
            for file in working_set.files() {
                if var.declaration_span.start >= file.1 && var.declaration_span.start < file.2 {
                    println!(
                        "{{\"file\": \"{}\", \"start\": {}, \"end\": {}}}",
                        file.0,
                        var.declaration_span.start - offset,
                        var.declaration_span.end - offset
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
        Some((Id::DeclId(decl_id), _)) => {
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

            let description = description.replace('\n', "\\n");
            let description = description.replace('\"', "\\\"");

            println!("{{\"hover\": \"{}\"}}", description);
        }
        Some((Id::VarId(var_id), _)) => {
            let var = working_set.get_variable(var_id);

            println!(
                "{{\"hover\": \"{}{}\"}}",
                if var.mutable { "mutable" } else { "" },
                var.ty
            );
        }
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
