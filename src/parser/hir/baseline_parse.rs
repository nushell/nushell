use crate::errors::ShellError;
use crate::parser::{hir, CommandRegistry, RawToken, Token, TokenNode};

// pub fn baseline_parse_token(
//     token_node: TokenNode,
//     _registry: &dyn CommandRegistry,
// ) -> Result<hir::Expression, ShellError> {
//     match token_node {
//         TokenNode::Token(token) => Ok(baseline_parse_single_token(token)),
//         TokenNode::Call(_call) => Err(ShellError::unimplemented("baseline_parse Call")),
//         TokenNode::Delimited(_delimited) => {
//             Err(ShellError::unimplemented("baseline_parse Delimited"))
//         }
//         TokenNode::Pipeline(_pipeline) => Err(ShellError::unimplemented("baseline_parse Pipeline")),
//         TokenNode::Path(_path) => Err(ShellError::unimplemented("baseline_parse Path")),
//     }
// }

pub fn baseline_parse_single_token(token: &Token, source: &str) -> hir::Expression {
    match *token.item() {
        RawToken::Integer(int) => hir::Expression::int(int, token.span),
        RawToken::Size(int, unit) => hir::Expression::size(int, unit, token.span),
        RawToken::String(span) => hir::Expression::string(span, token.span),
        RawToken::Variable(span) if span.slice(source) == "it" => {
            hir::Expression::it_variable(span, token.span)
        }
        RawToken::Variable(span) => hir::Expression::variable(span, token.span),
        RawToken::Bare => hir::Expression::bare(token.span),
    }
}
