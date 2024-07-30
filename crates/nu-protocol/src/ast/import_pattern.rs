use serde::{Deserialize, Serialize};

use crate::{ModuleId, Span, VarId};
use std::collections::HashSet;

/// possible patterns after the first module level in an [`ImportPattern`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportPatternMember {
    /// Wildcard import of items
    Glob { span: Span },
    /// single specific module or item
    Name { name: Vec<u8>, span: Span },
    /// list of items
    List { names: Vec<(Vec<u8>, Span)> },
}

impl ImportPatternMember {
    pub fn span(&self) -> Span {
        match self {
            ImportPatternMember::Glob { span } | ImportPatternMember::Name { span, .. } => *span,
            ImportPatternMember::List { names } => {
                let first = names
                    .first()
                    .map(|&(_, span)| span)
                    .unwrap_or(Span::unknown());

                let last = names
                    .last()
                    .map(|&(_, span)| span)
                    .unwrap_or(Span::unknown());

                Span::append(first, last)
            }
        }
    }
}

/// The first item of a `use` statement needs to specify an explicit module
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPatternHead {
    pub name: Vec<u8>,
    pub id: Option<ModuleId>,
    pub span: Span,
}

/// The pattern specifying modules in a `use` statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPattern {
    pub head: ImportPatternHead,
    pub members: Vec<ImportPatternMember>,
    // communicate to eval which decls/aliases were hidden during `parse_hide()` so it does not
    // interpret these as env var names:
    pub hidden: HashSet<Vec<u8>>,
    // information for the eval which const values to put into stack as variables
    pub constants: Vec<VarId>,
}

impl ImportPattern {
    pub fn new() -> Self {
        ImportPattern {
            head: ImportPatternHead {
                name: vec![],
                id: None,
                span: Span::unknown(),
            },
            members: vec![],
            hidden: HashSet::new(),
            constants: vec![],
        }
    }

    pub fn span(&self) -> Span {
        Span::append(
            self.head.span,
            self.members
                .last()
                .map(ImportPatternMember::span)
                .unwrap_or(self.head.span),
        )
    }

    pub fn with_hidden(self, hidden: HashSet<Vec<u8>>) -> Self {
        ImportPattern {
            head: self.head,
            members: self.members,
            hidden,
            constants: self.constants,
        }
    }
}

impl Default for ImportPattern {
    fn default() -> Self {
        Self::new()
    }
}
