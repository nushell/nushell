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
            SyntaxShape::Any => "any",
            SyntaxShape::String => "string",
            SyntaxShape::Member => "member",
            SyntaxShape::ColumnPath => "column path",
            SyntaxShape::Number => "number",
            SyntaxShape::Range => "range",
            SyntaxShape::Int => "integer",
            SyntaxShape::Path => "file path",
            SyntaxShape::Pattern => "pattern",
            SyntaxShape::Block => "block",
        })
    }
}
