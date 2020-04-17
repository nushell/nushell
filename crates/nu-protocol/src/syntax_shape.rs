use nu_source::{b, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};

/// The syntactic shapes that values must match to be passed into a command. You can think of this as the type-checking that occurs when you call a function.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SyntaxShape {
    /// Any syntactic form is allowed
    Any,
    /// Strings and string-like bare words are allowed
    String,
    /// A dotted path to navigate the table
    ColumnPath,
    /// A dotted path to navigate the table (including variable)
    FullColumnPath,
    /// Only a numeric (integer or decimal) value is allowed
    Number,
    /// A range is allowed (eg, `1..3`)
    Range,
    /// Only an integer value is allowed
    Int,
    /// A filepath is allowed
    Path,
    /// A glob pattern is allowed, eg `foo*`
    Pattern,
    /// A block is allowed, eg `{start this thing}`
    Block,
    /// A table is allowed, eg `[first second]`
    Table,
    /// A unit value is allowed, eg `10kb`
    Unit,
    /// An operator
    Operator,
    /// A math expression, eg `foo > 1`
    Math,
}

impl PrettyDebug for SyntaxShape {
    /// Prepare SyntaxShape for pretty-printing
    fn pretty(&self) -> DebugDocBuilder {
        b::kind(match self {
            SyntaxShape::Any => "any",
            SyntaxShape::String => "string",
            SyntaxShape::FullColumnPath => "column path (with variable)",
            SyntaxShape::ColumnPath => "column path",
            SyntaxShape::Number => "number",
            SyntaxShape::Range => "range",
            SyntaxShape::Int => "integer",
            SyntaxShape::Path => "file path",
            SyntaxShape::Pattern => "pattern",
            SyntaxShape::Block => "block",
            SyntaxShape::Table => "table",
            SyntaxShape::Unit => "unit",
            SyntaxShape::Operator => "operator",
            SyntaxShape::Math => "condition",
        })
    }
}
