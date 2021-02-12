use nu_protocol::hir::*;
use nu_source::{Span, Spanned, SpannedItem};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LocationType {
    Command,
    Flag(String),                             // command name
    Argument(Option<String>, Option<String>), // command name, argument name
    Variable,
}

pub type CompletionLocation = Spanned<LocationType>;

// TODO The below is very similar to shapes / expression_to_flat_shape. Check back October 2020
//      to see if we're close enough to just make use of those.

struct Flatten<'s> {
    line: &'s str,
    command: Option<String>,
    flag: Option<String>,
}

impl<'s> Flatten<'s> {
    /// Converts a SpannedExpression into a completion location for use in NuCompleter
    fn expression(&self, e: &SpannedExpression) -> Vec<CompletionLocation> {
        match &e.expr {
            Expression::Block(block) => self.completion_locations(block),
            Expression::Invocation(block) => self.completion_locations(block),
            Expression::List(exprs) => exprs.iter().flat_map(|v| self.expression(v)).collect(),
            Expression::Table(headers, cells) => headers
                .iter()
                .flat_map(|v| self.expression(v))
                .chain(
                    cells
                        .iter()
                        .flat_map(|v| v.iter().flat_map(|v| self.expression(v))),
                )
                .collect(),
            Expression::Command => vec![LocationType::Command.spanned(e.span)],
            Expression::Path(path) => self.expression(&path.head),
            Expression::Variable(_, _) => vec![LocationType::Variable.spanned(e.span)],

            Expression::Boolean(_)
            | Expression::FilePath(_)
            | Expression::Literal(Literal::ColumnPath(_))
            | Expression::Literal(Literal::GlobPattern(_))
            | Expression::Literal(Literal::Number(_))
            | Expression::Literal(Literal::Size(_, _))
            | Expression::Literal(Literal::String(_)) => {
                vec![
                    LocationType::Argument(self.command.clone(), self.flag.clone()).spanned(e.span),
                ]
            }

            Expression::Binary(binary) => {
                let mut result = Vec::new();
                result.append(&mut self.expression(&binary.left));
                result.append(&mut self.expression(&binary.right));
                result
            }
            Expression::Range(range) => {
                let mut result = Vec::new();
                if let Some(left) = &range.left {
                    result.append(&mut self.expression(left));
                }
                if let Some(right) = &range.right {
                    result.append(&mut self.expression(right));
                }
                result
            }

            Expression::ExternalWord
            | Expression::ExternalCommand(_)
            | Expression::Synthetic(_)
            | Expression::Literal(Literal::Operator(_))
            | Expression::Literal(Literal::Bare(_))
            | Expression::Garbage => Vec::new(),
        }
    }

    fn internal_command(&self, internal: &InternalCommand) -> Vec<CompletionLocation> {
        let mut result = Vec::new();

        match internal.args.head.expr {
            Expression::Command => {
                result.push(LocationType::Command.spanned(internal.name_span));
            }
            Expression::Literal(Literal::String(_)) => {
                result.push(LocationType::Command.spanned(internal.name_span));
            }
            _ => (),
        }

        if let Some(positionals) = &internal.args.positional {
            let mut positionals = positionals.iter();

            if internal.name == "run_external" {
                if let Some(external_command) = positionals.next() {
                    result.push(LocationType::Command.spanned(external_command.span));
                }
            }

            result.extend(positionals.flat_map(|positional| match positional.expr {
                Expression::Garbage => {
                    let garbage = positional.span.slice(self.line);
                    let location = if garbage.starts_with('-') {
                        LocationType::Flag(internal.name.clone())
                    } else {
                        // TODO we may be able to map this to the name of a positional,
                        //      but we'll need a signature
                        LocationType::Argument(Some(internal.name.clone()), None)
                    };

                    vec![location.spanned(positional.span)]
                }

                _ => self.expression(positional),
            }));
        }

        if let Some(named) = &internal.args.named {
            for (name, kind) in &named.named {
                match kind {
                    NamedValue::PresentSwitch(span) => {
                        result.push(LocationType::Flag(internal.name.clone()).spanned(*span));
                    }

                    NamedValue::Value(span, expr) => {
                        result.push(LocationType::Flag(internal.name.clone()).spanned(*span));
                        result.append(&mut self.with_flag(name.clone()).expression(expr));
                    }

                    _ => (),
                }
            }
        }

        result
    }

    fn pipeline(&self, pipeline: &Pipeline) -> Vec<CompletionLocation> {
        let mut result = Vec::new();

        for command in &pipeline.list {
            match command {
                ClassifiedCommand::Internal(internal) => {
                    let engine = self.with_command(internal.name.clone());
                    result.append(&mut engine.internal_command(internal));
                }

                ClassifiedCommand::Expr(expr) => result.append(&mut self.expression(expr)),
                _ => (),
            }
        }

        result
    }

    /// Flattens the block into a Vec of completion locations
    pub fn completion_locations(&self, block: &Block) -> Vec<CompletionLocation> {
        block
            .block
            .iter()
            .flat_map(|g| g.pipelines.iter().flat_map(|v| self.pipeline(v)))
            .collect()
    }

    pub fn new(line: &'s str) -> Flatten<'s> {
        Flatten {
            line,
            command: None,
            flag: None,
        }
    }

    pub fn with_command(&self, command: String) -> Flatten<'s> {
        Flatten {
            line: self.line,
            command: Some(command),
            flag: None,
        }
    }

    pub fn with_flag(&self, flag: String) -> Flatten<'s> {
        Flatten {
            line: self.line,
            command: self.command.clone(),
            flag: Some(flag),
        }
    }
}

/// Characters that precede a command name
const BEFORE_COMMAND_CHARS: &[char] = &['|', '(', ';'];

/// Determines the completion location for a given block at the given cursor position
pub fn completion_location(line: &str, block: &Block, pos: usize) -> Vec<CompletionLocation> {
    let completion_engine = Flatten::new(line);
    let locations = completion_engine.completion_locations(block);

    if locations.is_empty() {
        vec![LocationType::Command.spanned(Span::unknown())]
    } else {
        let mut command = None;
        let mut prev = None;
        for loc in locations {
            // We don't use span.contains because we want to include the end. This handles the case
            // where the cursor is just after the text (i.e., no space between cursor and text)
            if loc.span.start() <= pos && pos <= loc.span.end() {
                // The parser sees the "-" in `cmd -` as an argument, but the user is likely
                // expecting a flag.
                return match loc.item {
                    LocationType::Argument(ref cmd, _) => {
                        if loc.span.slice(line) == "-" {
                            let cmd = cmd.clone();
                            let span = loc.span;
                            vec![
                                loc,
                                LocationType::Flag(cmd.unwrap_or_default()).spanned(span),
                            ]
                        } else {
                            vec![loc]
                        }
                    }
                    _ => vec![loc],
                };
            } else if pos < loc.span.start() {
                break;
            }

            if let LocationType::Command = loc.item {
                command = Some(String::from(loc.span.slice(line)));
            }

            prev = Some(loc);
        }

        if let Some(prev) = prev {
            // Cursor is between locations (or at the end). Look at the line to see if the cursor
            // is after some character that would imply we're in the command position.
            let start = prev.span.end();
            if line[start..pos].contains(BEFORE_COMMAND_CHARS) {
                vec![LocationType::Command.spanned(Span::new(pos, pos))]
            } else {
                // TODO this should be able to be mapped to a command
                vec![LocationType::Argument(command, None).spanned(Span::new(pos, pos))]
            }
        } else {
            // Cursor is before any possible completion location, so must be a command
            vec![LocationType::Command.spanned(Span::unknown())]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use nu_parser::{classify_block, lex, parse_block, ParserScope};
    use nu_protocol::{Signature, SyntaxShape};

    #[derive(Clone, Debug)]
    struct VecRegistry(Vec<Signature>);

    impl From<Vec<Signature>> for VecRegistry {
        fn from(v: Vec<Signature>) -> Self {
            VecRegistry(v)
        }
    }

    impl ParserScope for VecRegistry {
        fn has_signature(&self, name: &str) -> bool {
            self.0.iter().any(|v| v.name == name)
        }

        fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature> {
            self.0.iter().find(|v| v.name == name).map(Clone::clone)
        }

        fn get_alias(&self, _name: &str) -> Option<Vec<Spanned<String>>> {
            None
        }

        fn add_alias(&self, _name: &str, _replacement: Vec<Spanned<String>>) {
            todo!()
        }

        fn add_definition(&self, _block: Block) {}

        fn get_definitions(&self) -> Vec<Block> {
            vec![]
        }

        fn enter_scope(&self) {}

        fn exit_scope(&self) {}
    }

    mod completion_location {
        use super::*;

        use nu_parser::ParserScope;

        fn completion_location(
            line: &str,
            scope: &dyn ParserScope,
            pos: usize,
        ) -> Vec<LocationType> {
            let (tokens, _) = lex(line, 0);
            let (lite_block, _) = parse_block(tokens);

            scope.enter_scope();
            let (block, _) = classify_block(&lite_block, scope);
            scope.exit_scope();

            super::completion_location(line, &block, pos)
                .into_iter()
                .map(|v| v.item)
                .collect()
        }

        #[test]
        fn completes_internal_command_names() {
            let registry: VecRegistry =
                vec![Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")].into();
            let line = "echo 1 | echo 2";

            assert_eq!(
                completion_location(line, &registry, 10),
                vec![LocationType::Command],
            );
        }

        #[test]
        fn completes_external_command_names() {
            let registry: VecRegistry = Vec::new().into();
            let line = "echo 1 | echo 2";

            assert_eq!(
                completion_location(line, &registry, 10),
                vec![LocationType::Command],
            );
        }

        #[test]
        fn completes_command_names_when_cursor_immediately_after_command_name() {
            let registry: VecRegistry = Vec::new().into();
            let line = "echo 1 | echo 2";

            assert_eq!(
                completion_location(line, &registry, 4),
                vec![LocationType::Command],
            );
        }

        #[test]
        fn completes_variables() {
            let registry: VecRegistry = Vec::new().into();
            let line = "echo $nu.env.";

            assert_eq!(
                completion_location(line, &registry, 13),
                vec![LocationType::Variable],
            );
        }

        #[test]
        fn completes_flags() {
            let registry: VecRegistry = vec![Signature::build("du")
                .switch("recursive", "the values to echo", None)
                .rest(SyntaxShape::Any, "blah")]
            .into();

            let line = "du --recurs";

            assert_eq!(
                completion_location(line, &registry, 7),
                vec![LocationType::Flag("du".to_string())],
            );
        }

        #[test]
        fn completes_incomplete_nested_structure() {
            let registry: VecRegistry = vec![Signature::build("sys")].into();
            let line = "echo $(sy";

            assert_eq!(
                completion_location(line, &registry, 8),
                vec![LocationType::Command],
            );
        }

        #[test]
        fn has_correct_command_name_for_argument() {
            let registry: VecRegistry = vec![Signature::build("cd")].into();
            let line = "cd ";

            assert_eq!(
                completion_location(line, &registry, 3),
                vec![LocationType::Argument(Some("cd".to_string()), None)],
            );
        }

        #[test]
        fn completes_flags_with_just_a_single_hyphen() {
            let registry: VecRegistry = vec![Signature::build("du")
                .switch("recursive", "the values to echo", None)
                .rest(SyntaxShape::Any, "blah")]
            .into();

            let line = "du -";

            assert_eq!(
                completion_location(line, &registry, 3),
                vec![
                    LocationType::Argument(Some("du".to_string()), None),
                    LocationType::Flag("du".to_string()),
                ],
            );
        }

        #[test]
        fn completes_arguments() {
            let registry: VecRegistry =
                vec![Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")].into();
            let line = "echo 1 | echo 2";

            assert_eq!(
                completion_location(line, &registry, 6),
                vec![LocationType::Argument(Some("echo".to_string()), None)],
            );
        }
    }
}
