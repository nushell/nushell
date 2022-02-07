use quickcheck_macros::quickcheck;

mod commands;
mod format_conversions;

use nu_engine::EvaluationContext;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(&data, 0, nu_parser::NewlineMode::Normal);
    let (lite_block, err2) = nu_parser::parse_block(tokens);

    if err.is_none() && err2.is_none() {
        let context = EvaluationContext::basic();
        let _ = nu_parser::classify_block(&lite_block, &context.scope);
    }
    true
}
