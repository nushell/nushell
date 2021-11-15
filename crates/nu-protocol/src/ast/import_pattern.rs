use crate::{span, Span};

#[derive(Debug, Clone)]
pub enum ImportPatternMember {
    Glob { span: Span },
    Name { name: Vec<u8>, span: Span },
    List { names: Vec<(Vec<u8>, Span)> },
}

#[derive(Debug, Clone)]
pub struct ImportPatternHead {
    pub name: Vec<u8>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportPattern {
    pub head: ImportPatternHead,
    pub members: Vec<ImportPatternMember>,
}

impl ImportPattern {
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
}
