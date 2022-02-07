<<<<<<< HEAD
=======
use nu_command::create_default_context;
use nu_protocol::engine::StateWorkingSet;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
use quickcheck_macros::quickcheck;

mod commands;
mod format_conversions;

<<<<<<< HEAD
use nu_engine::EvaluationContext;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(&data, 0, nu_parser::NewlineMode::Normal);
    let (lite_block, err2) = nu_parser::parse_block(tokens);

    if err.is_none() && err2.is_none() {
        let context = EvaluationContext::basic();
        let _ = nu_parser::classify_block(&lite_block, &context.scope);
=======
// use nu_engine::EvaluationContext;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(data.as_bytes(), 0, b"", b"", true);
    let (lite_block, err2) = nu_parser::lite_parse(&tokens);

    if err.is_none() && err2.is_none() {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        let context = create_default_context(cwd);
        {
            let mut working_set = StateWorkingSet::new(&context);
            working_set.add_file("quickcheck".into(), data.as_bytes());

            let _ = nu_parser::parse_block(&mut working_set, &lite_block, false);
        }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }
    true
}
