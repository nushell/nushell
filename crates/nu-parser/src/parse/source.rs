use crate::{lex::tokens::LiteCommand, ParserScope};
use nu_errors::{ArgumentError, ParseError};
use nu_path::{canonicalize, canonicalize_with};
use nu_protocol::hir::{Expression, InternalCommand};

use std::path::Path;

use nu_source::SpannedItem;

pub fn parse_source_internal(
    lite_cmd: &LiteCommand,
    command: &InternalCommand,
    scope: &dyn ParserScope,
) -> Result<(), ParseError> {
    if lite_cmd.parts.len() != 2 {
        return Err(ParseError::argument_error(
            lite_cmd.parts[0].clone(),
            ArgumentError::MissingMandatoryPositional("a path for sourcing".into()),
        ));
    }

    if lite_cmd.parts[1].item.starts_with('$') {
        return Err(ParseError::mismatch(
            "a filepath constant",
            lite_cmd.parts[1].clone(),
        ));
    }

    // look for source files in lib dirs first
    // if not files are found, try the current path
    // first file found wins.
    find_source_file(lite_cmd, command, scope)
}

fn find_source_file(
    lite_cmd: &LiteCommand,
    command: &InternalCommand,
    scope: &dyn ParserScope,
) -> Result<(), ParseError> {
    let (file, file_span) = if let Some(ref positional_args) = command.args.positional {
        if let Expression::FilePath(ref p) = positional_args[0].expr {
            (p.as_path(), &positional_args[0].span)
        } else {
            (Path::new(&lite_cmd.parts[1].item), &lite_cmd.parts[1].span)
        }
    } else {
        (Path::new(&lite_cmd.parts[1].item), &lite_cmd.parts[1].span)
    };

    let lib_dirs = nu_data::config::config(nu_source::Tag::unknown())
        .ok()
        .as_ref()
        .map(|configuration| match configuration.get("lib_dirs") {
            Some(paths) => paths
                .table_entries()
                .cloned()
                .map(|path| path.as_string())
                .collect(),
            None => vec![],
        });

    if let Some(dir) = lib_dirs {
        for lib_path in dir.into_iter().flatten() {
            let path = if let Ok(p) = canonicalize_with(&file, lib_path) {
                p
            } else {
                continue;
            };

            if let Ok(contents) = std::fs::read_to_string(&path) {
                return parse(&contents, 0, scope);
            }
        }
    }

    let path = canonicalize(&file).map_err(|e| {
        ParseError::general_error(
            format!("Can't load source file. Reason: {}", e.to_string()),
            "Can't load this file".spanned(file_span),
        )
    })?;

    let contents = std::fs::read_to_string(&path);

    match contents {
        Ok(contents) => parse(&contents, 0, scope),
        Err(e) => Err(ParseError::general_error(
            format!("Can't load source file. Reason: {}", e.to_string()),
            "Can't load this file".spanned(file_span),
        )),
    }
}

pub fn parse(input: &str, span_offset: usize, scope: &dyn ParserScope) -> Result<(), ParseError> {
    if let (_, Some(parse_error)) = super::parse(input, span_offset, scope) {
        Err(parse_error)
    } else {
        Ok(())
    }
}
