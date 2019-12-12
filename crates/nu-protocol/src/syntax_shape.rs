use nu_source::{b, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SyntaxShape {
    Any,
    String,
    Member,
    ColumnPath,
    Number,
    Range,
    Int,
    Path,
    Pattern,
    Block,
}

impl PrettyDebug for SyntaxShape {
    fn pretty(&self) -> DebugDocBuilder {
        b::kind(match self {
            SyntaxShape::Any => "any shape",
            SyntaxShape::String => "string shape",
            SyntaxShape::Member => "member shape",
            SyntaxShape::ColumnPath => "column path shape",
            SyntaxShape::Number => "number shape",
            SyntaxShape::Range => "range shape",
            SyntaxShape::Int => "integer shape",
            SyntaxShape::Path => "file path shape",
            SyntaxShape::Pattern => "pattern shape",
            SyntaxShape::Block => "block shape",
        })
    }
}
