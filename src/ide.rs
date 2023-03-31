use miette::IntoDiagnostic;
use nu_cli::report_error;
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    DeclId, ShellError, Span, Value, VarId,
};

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
                println!("{:?}", item.1);

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

pub fn goto_def(engine_state: &mut EngineState, file_path: &String, location: &Value) -> String {
    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::DeclId(decl_id), offset)) => {
            let result = working_set.get_decl(decl_id);
            if let Some(block_id) = result.get_block_id() {
                let block = working_set.get_block(block_id);
                if let Some(span) = &block.span {
                    println!("Declaration at: {:?}", span.start - offset);
                }
            }
        }
        Some((Id::VarId(var_id), offset)) => {
            let working_set = StateWorkingSet::new(engine_state);
            let var = working_set.get_variable(var_id);
            println!(
                "Variable created at: {:?}",
                var.declaration_span.start - offset
            );
        }
        _ => {}
    }

    "".into()
}

pub fn hover(engine_state: &mut EngineState, file_path: &String, location: &Value) -> String {
    let (file, mut working_set) = read_in_file(engine_state, file_path);

    match find_id(&mut working_set, file_path, &file, location) {
        Some((Id::DeclId(decl_id), _)) => {
            let decl = working_set.get_decl(decl_id);

            println!("Signature: {}", decl.signature())
        }
        Some((Id::VarId(var_id), _)) => {
            let working_set = StateWorkingSet::new(engine_state);
            let var = working_set.get_variable(var_id);

            println!(
                "Variable type: {}{}",
                if var.mutable { "mutable" } else { "" },
                var.ty
            );
        }
        _ => {}
    }

    "".into()
}
