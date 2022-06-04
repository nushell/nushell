use serde::{Deserialize, Serialize};

use crate::{span, ModuleId, Span};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportPatternMember {
    Glob { span: Span },
    Name { name: Vec<u8>, span: Span },
    List { names: Vec<(Vec<u8>, Span)> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPatternHead {
    pub name: Vec<u8>,
    pub id: Option<ModuleId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPattern {
    pub head: ImportPatternHead,
    pub members: Vec<ImportPatternMember>,
    // communicate to eval which decls/aliases were hidden during `parse_hide()` so it does not
    // interpret these as env var names:
    pub hidden: HashSet<Vec<u8>>,
}

impl ImportPattern {
    pub fn new() -> Self {
        ImportPattern {
            head: ImportPatternHead {
                name: vec![],
                id: None,
                span: Span { start: 0, end: 0 },
            },
            members: vec![],
            hidden: HashSet::new(),
        }
    }

    pub fn span(&self) -> Span {
        let mut spans = vec![self.head.span];

        for member in &self.members {
            match member {
                ImportPatternMember::Glob { span } => spans.push(*span),
                ImportPatternMember::Name { name: _, span } => spans.push(*span),
                ImportPatternMember::List { names } => {
                    for (_, span) in names {
                        spans.push(*span);
                    }
                }
            }
        }

        span(&spans)
    }

    pub fn with_hidden(self, hidden: HashSet<Vec<u8>>) -> Self {
        ImportPattern {
            head: self.head,
            members: self.members,
            hidden,
        }
    }
}

impl Default for ImportPattern {
    fn default() -> Self {
        Self::new()
    }
}
