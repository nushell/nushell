use derive_new::new;
use itertools::Itertools;
use std::fmt;

use nu_source::{HasSpan, Span, Spanned, SpannedItem};

use super::token_group::TokenBuilder;

#[derive(Debug, Clone, PartialEq, is_enum_variant)]
pub enum TokenContents {
    /// A baseline token is an atomic chunk of source code. This means that the
    /// token contains the entirety of string literals, as well as the entirety
    /// of sections delimited by paired delimiters.
    ///
    /// For example, if the token begins with `{`, the baseline token continues
    /// until the closing `}` (after taking comments and string literals into
    /// consideration).
    Baseline(String),
    Comment(LiteComment),
    Pipe,
    Semicolon,
    EOL,
}

impl fmt::Display for TokenContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenContents::Baseline(base) => write!(f, "{}", base),
            TokenContents::Comment(comm) => write!(f, "{}", comm),
            TokenContents::Pipe => write!(f, "|"),
            TokenContents::Semicolon => write!(f, ";"),
            TokenContents::EOL => write!(f, "\\n"),
        }
    }
}

pub type CommandBuilder = TokenBuilder<Spanned<String>>;
pub type CommentsBuilder = TokenBuilder<LiteComment>;
pub type PipelineBuilder = TokenBuilder<LiteCommand>;
pub type GroupBuilder = TokenBuilder<PipelineBuilder>;

/// A LiteComment is a line comment. It begins with `#` and continues until (but not including) the
/// next newline.
///
/// It remembers any leading whitespace, which is used in later processing steps to strip off
/// leading whitespace for an entire comment block when it is associated with a definition.
#[derive(Debug, PartialEq, Clone)]
pub struct LiteComment {
    leading_ws: Option<Spanned<String>>,
    rest: Spanned<String>,
}

impl LiteComment {
    pub fn new(string: impl Into<Spanned<String>>) -> LiteComment {
        LiteComment {
            leading_ws: None,
            rest: string.into(),
        }
    }

    pub fn new_with_ws(
        ws: impl Into<Spanned<String>>,
        comment: impl Into<Spanned<String>>,
    ) -> LiteComment {
        LiteComment {
            leading_ws: Some(ws.into()),
            rest: comment.into(),
        }
    }

    pub fn unindent(&self, excluded_spaces: usize) -> LiteComment {
        match &self.leading_ws {
            // If there's no leading whitespace, there's no whitespace to exclude
            None => self.clone(),
            Some(Spanned { item, span }) => {
                // If the number of spaces to exclude is larger than the amount of whitespace we
                // have, there's no whitespace to move into the comment body.
                if excluded_spaces > item.len() {
                    self.clone()
                } else {
                    // If there are no spaces to exclude, prepend all of the leading_whitespace to
                    // the comment body.
                    if excluded_spaces == 0 {
                        let rest_span = self.span();
                        let rest = format!("{}{}", item, self.rest.item).spanned(rest_span);
                        return LiteComment {
                            leading_ws: None,
                            rest,
                        };
                    }

                    // Pull off excluded_spaces number of spaces, and create a new Spanned<String>
                    // for that whitespace. Any remaining spaces will be added to the comment.
                    let excluded_ws = item[..excluded_spaces]
                        .to_string()
                        .spanned(Span::new(span.start(), span.start() + excluded_spaces));

                    let included_ws = &item[excluded_spaces..];
                    let rest_start = span.start() + excluded_spaces;
                    let rest_span = Span::new(rest_start, rest_start + self.rest.len());

                    let rest = format!("{}{}", included_ws, self.rest.item).spanned(rest_span);

                    LiteComment {
                        leading_ws: Some(excluded_ws),
                        rest,
                    }
                }
            }
        }
    }

    pub fn ws_len(&self) -> usize {
        match &self.leading_ws {
            None => 0,
            Some(ws) => ws.item.len(),
        }
    }

    pub(crate) fn trim(&self) -> Spanned<String> {
        let trimmed = self.rest.trim();

        trimmed.to_string().spanned(Span::new(
            self.rest.span().start(),
            self.rest.span().start() + trimmed.len(),
        ))
    }
}

impl fmt::Display for LiteComment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.leading_ws {
            None => write!(f, "#{}", self.rest.item),
            Some(leading) => write!(f, "#{}{}", leading.item, self.rest.item),
        }
    }
}

impl HasSpan for LiteComment {
    fn span(&self) -> Span {
        match &self.leading_ws {
            None => self.rest.span(),
            Some(leading) => leading.span().until(self.rest.span()),
        }
    }
}

/// A `LiteCommand` is a list of words that will get meaning when processed by
/// the parser.
#[derive(Debug, Default, Clone)]
pub struct LiteCommand {
    pub parts: Vec<Spanned<String>>,
    /// Preceding comments.
    pub comments: Option<Vec<LiteComment>>,
}

impl HasSpan for LiteCommand {
    fn span(&self) -> Span {
        Span::from_list(&self.parts)
    }
}

impl LiteCommand {
    pub fn comments_joined(&self) -> String {
        match &self.comments {
            None => "".to_string(),
            Some(text) => text.iter().map(|s| s.trim().item).join("\n"),
        }
    }
}

/// A `LitePipeline` is a series of `LiteCommand`s, separated by `|`.
#[derive(Debug, Clone, new)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

impl HasSpan for LitePipeline {
    fn span(&self) -> Span {
        Span::from_list(&self.commands)
    }
}

/// A `LiteGroup` is a series of `LitePipeline`s, separated by `;`.
#[derive(Debug, Clone, new)]
pub struct LiteGroup {
    pub pipelines: Vec<LitePipeline>,
}

impl From<GroupBuilder> for LiteGroup {
    fn from(group: GroupBuilder) -> Self {
        LiteGroup::new(group.map(|p| LitePipeline::new(p.into())))
    }
}

impl HasSpan for LiteGroup {
    fn span(&self) -> Span {
        Span::from_list(&self.pipelines)
    }
}

/// A `LiteBlock` is a series of `LiteGroup`s, separated by newlines.
#[derive(Debug, Clone, new)]
pub struct LiteBlock {
    pub block: Vec<LiteGroup>,
}

impl HasSpan for LiteBlock {
    fn span(&self) -> Span {
        Span::from_list(&self.block)
    }
}
