//! The layout checks nu's keyword parsers make on a statement's items before reading them:
//! a definition's name with its parameters joined to it (`detect_params_in_name`), an alias
//! without its name or `=` (`check_alias_name`, `report_incomplete_alias`) and a type alias
//! that is no `name = ...` (`type_alias_layout_error`).

use nu_protocol::{DeclId, ParseError, Span};

use super::Lower;

impl Lower<'_, '_, '_> {
    /// The error for a definition's name item at `name_span` with a `[` or `(` in it, its
    /// parameters joined to it, for the keyword command `decl_id` (`detect_params_in_name`).
    pub(super) fn params_in_name(&self, name_span: Span, decl_id: DeclId) -> Option<ParseError> {
        let offset = self.text_at(name_span).find(['[', '('])?;
        // Just before the bracket, where the space belongs.
        let at = (name_span.start + offset).saturating_sub(1);
        Some(ParseError::LabeledErrorWithHelp {
            error: "no space between name and parameters".into(),
            label: "expected space".into(),
            help: format!(
                "consider adding a space between the `{}` command's name and its parameters",
                self.working_set.get_decl(decl_id).name()
            ),
            span: Span::new(at, at),
        })
    }

    /// The item of an `alias` statement's items `spans` (`alias` or `export alias` first) that
    /// is out of place, reported: a `=` where its name belongs, or what stands where its `=`
    /// belongs (`check_alias_name`).
    pub(super) fn misplaced_alias_item(&mut self, spans: &[Span]) -> Option<Span> {
        let command_len = match spans.first() {
            Some(first) if self.text_at(*first) == "export" => 2,
            Some(_) => 1,
            None => return None,
        };
        let keyword = || {
            self.text_at(Span::concat(&spans[..command_len]))
                .to_string()
        };
        let (problem, label, at) = match spans.len() {
            len if len == command_len => return None,
            len if len < command_len + 3 => {
                if self.text_at(spans[command_len]) != "=" {
                    return None;
                }
                ("missing name", "missing name", spans[command_len])
            }
            _ if self.text_at(spans[command_len + 1]) != "=" => {
                ("missing sign", "missing equal sign", spans[command_len + 1])
            }
            _ => return None,
        };
        let error =
            ParseError::AssignmentMismatch(format!("{} {problem}", keyword()), label.into(), at);
        self.working_set.error(error);
        Some(at)
    }

    /// What `parse_alias` reports for an `alias` statement that declares nothing, `command`
    /// its keyword items (`alias`, or `export alias`) and `args` the items after them: one item
    /// with an `=` in it (`alias ll=ls`), or fewer than three items (`report_incomplete_alias`).
    pub(super) fn incomplete_alias(&mut self, command: &[Span], args: &[Span]) {
        if let [arg] = args
            && let Some((name, value)) = self.text_at(*arg).split_once('=')
        {
            let name = if name.is_empty() { "{name}" } else { name };
            let value = if value.is_empty() {
                "{initial_value}"
            } else {
                value
            };
            self.working_set.error(ParseError::IncorrectValue(
                "alias argument".into(),
                *arg,
                format!("Make sure to put spaces around '=': alias {name} = {value}"),
            ));
        } else if args.len() < 3 {
            self.working_set.error(ParseError::IncorrectValue(
                "Incomplete alias".into(),
                Span::concat(command),
                "incomplete alias".into(),
            ));
        }
    }

    /// The error of a type alias whose items after its keyword at `head` (`type` or `export
    /// type`), `rest`, are no `name = ...`: no name, a `=` first, one item (`T=int`, `ls`) or a
    /// second item that is no `=` (`type_alias_layout_error`).
    pub(super) fn type_alias_layout(&self, head: Span, rest: &[Span]) -> Option<ParseError> {
        let keyword = self.text_at(head);
        // `type ls` is how Bash and cmd.exe look up a command, so a bare `type` without `=`
        // also points to `which`.
        let missing_equals = |name_span: Span| {
            let name = self.text_at(name_span);
            let lookup = if keyword == "type" {
                format!(
                    " To see what the command `{name}` is, use `which {name}`. To run an external `type` program, use `^type {name}`."
                )
            } else {
                String::new()
            };
            let help =
                format!("`{keyword}` defines a type alias: `{keyword} {name} = <type>`.{lookup}");
            ParseError::TypeAliasMissingEquals {
                help,
                span: name_span,
            }
        };
        match rest {
            [] => Some(ParseError::MissingPositional(
                "name".into(),
                Span::point(head.end),
                format!("{keyword} <name> = <type>"),
            )),
            [first, ..] if self.text_at(*first) == "=" => Some(ParseError::AssignmentMismatch(
                format!("{keyword} missing name"),
                "missing name".into(),
                *first,
            )),
            [word] => Some(match self.text_at(*word).split_once('=') {
                Some((name, ty)) => ParseError::IncorrectValue(
                    "type alias".into(),
                    *word,
                    format!("Make sure to put spaces around '=': {keyword} {name} = {ty}"),
                ),
                None => missing_equals(*word),
            }),
            [name, equals, ..] if self.text_at(*equals) != "=" => Some(missing_equals(*name)),
            _ => None,
        }
    }
}
