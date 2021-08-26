use crate::{lex::tokens::LiteCommand, ParserScope};
use nu_errors::{ArgumentError, ParseError};
use nu_path::expand_path;
use nu_protocol::hir::{Expression, InternalCommand};

use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;

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
    let file = if let Some(ref positional_args) = command.args.positional {
        if let Expression::FilePath(ref p) = positional_args[0].expr {
            p
        } else {
            Path::new(&lite_cmd.parts[1].item)
        }
    } else {
        Path::new(&lite_cmd.parts[1].item)
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
            let path = PathBuf::from(lib_path).join(&file);

            if let Ok(contents) =
                std::fs::read_to_string(&expand_path(Cow::Borrowed(path.as_path())))
            {
                return parse(&contents, 0, scope);
            }
        }
    }

    let path = Path::new(&file);

    let contents = std::fs::read_to_string(&expand_path(Cow::Borrowed(path)));

    match contents {
        Ok(contents) => parse(&contents, 0, scope),
        Err(_) => Err(ParseError::argument_error(
            lite_cmd.parts[1].clone(),
            ArgumentError::BadValue("can't load source file".into()),
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
