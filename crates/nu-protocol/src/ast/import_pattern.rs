use crate::Span;

#[derive(Debug, Clone)]
pub enum ImportPatternMember {
    Glob { span: Span },
    Name { name: Vec<u8>, span: Span },
}

#[derive(Debug, Clone)]
pub struct ImportPattern {
    pub head: Vec<u8>,
    pub members: Vec<ImportPatternMember>,
}
