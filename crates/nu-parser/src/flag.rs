use nu_errors::{ArgumentError, ParseError};
use nu_protocol::hir::InternalCommand;
use nu_protocol::NamedType;
use nu_source::{Span, Spanned, SpannedItem};

/// Match the available flags in a signature with what the user provided. This will check both long-form flags (--long) and shorthand flags (-l)
/// This also allows users to provide a group of shorthand flags (-la) that correspond to multiple shorthand flags at once.
pub fn get_flag_signature_spec(
    signature: &nu_protocol::Signature,
    cmd: &InternalCommand,
    arg: &Spanned<String>,
) -> (Vec<(String, NamedType)>, Option<ParseError>) {
    if arg.item.starts_with('-') {
        // It's a flag (or set of flags)
        let mut output = vec![];
        let mut error = None;

        let remainder: String = arg.item.chars().skip(1).collect();

        if remainder.starts_with('-') {
            // Long flag expected
            let mut remainder: String = remainder.chars().skip(1).collect();

            if remainder.contains('=') {
                let assignment: Vec<_> = remainder.split('=').collect();

                if assignment.len() != 2 {
                    error = Some(ParseError::argument_error(
                        cmd.name.to_string().spanned(cmd.name_span),
                        ArgumentError::InvalidExternalWord,
                    ));
                } else {
                    remainder = assignment[0].to_string();
                }
            }

            if let Some((named_type, _)) = signature.named.get(&remainder) {
                output.push((remainder.clone(), named_type.clone()));
            } else {
                error = Some(ParseError::argument_error(
                    cmd.name.to_string().spanned(cmd.name_span),
                    ArgumentError::UnexpectedFlag(arg.clone()),
                ));
            }
        } else {
            // Short flag(s) expected
            let mut starting_pos = arg.span.start() + 1;
            for c in remainder.chars() {
                let mut found = false;
                for (full_name, named_arg) in &signature.named {
                    if Some(c) == named_arg.0.get_short() {
                        found = true;
                        output.push((full_name.clone(), named_arg.0.clone()));
                        break;
                    }
                }

                if !found {
                    error = Some(ParseError::argument_error(
                        cmd.name.to_string().spanned(cmd.name_span),
                        ArgumentError::UnexpectedFlag(
                            arg.item
                                .clone()
                                .spanned(Span::new(starting_pos, starting_pos + c.len_utf8())),
                        ),
                    ));
                }

                starting_pos += c.len_utf8();
            }
        }

        (output, error)
    } else {
        // It's not a flag, so don't bother with it
        (vec![], None)
    }
}

#[cfg(test)]
mod tests {
    use super::get_flag_signature_spec;
    use crate::{lex, parse_block};
    use nu_protocol::{hir::InternalCommand, NamedType, Signature, SyntaxShape};
    use nu_source::{HasSpan, Span};

    fn bundle() -> Signature {
        Signature::build("bundle add")
            .switch("skip-install", "Adds the gem to the Gemfile but does not install it.", None)
            .named("group", SyntaxShape::String, "Specify the group(s) for the added gem. Multiple groups should be separated by commas.", Some('g'))
            .rest("rest", SyntaxShape::Any, "options")
    }

    #[test]
    fn parses_longform_flag_containing_equal_sign() {
        let input = "bundle add rails --group=development";
        let (tokens, _) = lex(input, 0, lex::lexer::NewlineMode::Normal);
        let (root_node, _) = parse_block(tokens);

        assert_eq!(root_node.block.len(), 1);
        assert_eq!(root_node.block[0].pipelines.len(), 1);
        assert_eq!(root_node.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(root_node.block[0].pipelines[0].commands[0].parts.len(), 4);

        let command_node = root_node.block[0].pipelines[0].commands[0].clone();
        let idx = 1;

        let (name, name_span) = (
            command_node.parts[0..(idx + 1)]
                .iter()
                .map(|x| x.item.clone())
                .collect::<Vec<String>>()
                .join(" "),
            Span::new(
                command_node.parts[0].span.start(),
                command_node.parts[idx].span.end(),
            ),
        );

        let mut internal = InternalCommand::new(name, name_span, command_node.span());

        let signature = bundle();

        internal.args.set_initial_flags(&signature);

        let (flags, err) = get_flag_signature_spec(&signature, &internal, &command_node.parts[3]);
        let (long_name, spec) = flags[0].clone();

        assert!(err.is_none());
        assert_eq!(long_name, "group".to_string());
        assert_eq!(spec.get_short(), Some('g'));

        match spec {
            NamedType::Optional(_, _) => {}
            _ => panic!("optional flag didn't parse succesfully"),
        }
    }
}
