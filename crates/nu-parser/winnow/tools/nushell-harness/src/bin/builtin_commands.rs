//! Print every command a fresh `nu` resolves at the top level, one per line
//! as `name<TAB>type<TAB>row-condition<TAB>signature`, sorted by name. `type` is
//! nu-protocol's `CommandType` (`Builtin`, `Keyword`, `Custom` for the
//! standard library's prelude); `row-condition` is `true` when the command's
//! first positional parameter is a row condition (`any`, `take while`);
//! `signature` is the command's parameters in nu's signature syntax with the
//! shapes the winnow parser reads (see [`signature_text`]), empty for a keyword.
//!
//! ```text
//! cargo run --release --bin builtin-commands
//! ```
//!
//! `winnow/tools/scripts/gen-builtin-commands.nu` turns the output into
//! `src/winnow/syntax/builtin_commands.rs`, so the parser knows exactly the commands this
//! harness's nu-parser knows.

use nu_protocol::{Signature, SyntaxShape, engine::CommandType};
use nushell_harness::engine;

pub fn main() {
    let engine_state = engine(true);
    for (name, decl_id) in engine_state.get_decls_sorted(false) {
        let decl = engine_state.get_decl(decl_id);
        let command_type = decl.command_type();
        let signature = decl.signature();
        let row_condition = signature
            .required_positional
            .first()
            .is_some_and(|positional| positional.shape == SyntaxShape::RowCondition);
        // A keyword's statement has a parser of its own.
        let signature = match command_type {
            CommandType::Keyword => String::new(),
            _ => signature_text(&signature),
        };
        println!(
            "{}\t{command_type:?}\t{row_condition}\t{signature}",
            String::from_utf8_lossy(&name)
        );
    }
}

/// The parameters of `signature` as a `[...]` signature item the winnow parser reads: the
/// positionals named `p0`, `p1`, ... (their names do not matter to a call), the rest
/// parameter, then the flags, each with its shape as [`shape_text`] writes it.
fn signature_text(signature: &Signature) -> String {
    let required = signature.required_positional.iter().map(|p| (p, ""));
    let optional = signature.optional_positional.iter().map(|p| (p, "?"));
    let mut params: Vec<String> = required
        .chain(optional)
        .enumerate()
        .map(|(index, (p, mark))| format!("p{index}{mark}: {}", shape_text(&p.shape)))
        .collect();
    if let Some(rest) = &signature.rest_positional {
        params.push(format!("...rest: {}", shape_text(&rest.shape)));
    }
    for flag in &signature.named {
        let name = match (flag.long_name(), flag.short) {
            (Some(long), Some(short)) => format!("--{long}(-{short})"),
            (Some(long), None) => format!("--{long}"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => continue,
        };
        // A signature refuses `bool` on a flag (it would be a switch); `oneof<bool>` reads the
        // value the same way.
        params.push(match &flag.arg {
            Some(SyntaxShape::Boolean) => format!("{name}: oneof<bool>"),
            Some(shape) => format!("{name}: {}", shape_text(shape)),
            None => name,
        });
    }
    format!("[{}]", params.join(", "))
}

/// A shape as the winnow parser's type parser reads it. A closure's parameter types, a record's
/// or table's columns, a completer and the shapes only keyword statements take (a row
/// condition, a block, a math expression, ...) are what a call's type checks or the keyword's
/// own parser use, not how the argument parses: those are left out or read as `any`.
fn shape_text(shape: &SyntaxShape) -> String {
    match shape {
        SyntaxShape::Binary => "binary".into(),
        SyntaxShape::Boolean => "bool".into(),
        SyntaxShape::CellPath => "cell-path".into(),
        SyntaxShape::Closure(_) => "closure".into(),
        SyntaxShape::DateTime => "datetime".into(),
        SyntaxShape::Directory => "directory".into(),
        SyntaxShape::Duration => "duration".into(),
        SyntaxShape::ExternalArgument => "external_arg".into(),
        SyntaxShape::Filepath => "path".into(),
        SyntaxShape::Filesize => "filesize".into(),
        SyntaxShape::Float => "float".into(),
        SyntaxShape::GlobPattern => "glob".into(),
        SyntaxShape::Int => "int".into(),
        SyntaxShape::List(inner) => format!("list<{}>", shape_text(inner)),
        SyntaxShape::Nothing => "nothing".into(),
        SyntaxShape::Number => "number".into(),
        SyntaxShape::OneOf(shapes) => {
            let shapes: Vec<String> = shapes.iter().map(shape_text).collect();
            format!("oneof<{}>", shapes.join(", "))
        }
        SyntaxShape::Range => "range".into(),
        SyntaxShape::Record(_) => "record".into(),
        SyntaxShape::String => "string".into(),
        SyntaxShape::Table(_) => "table".into(),
        _ => "any".into(),
    }
}
