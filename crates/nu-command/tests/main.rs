use nu_command::create_default_context;
use nu_protocol::engine::StateWorkingSet;
use quickcheck_macros::quickcheck;

mod commands;
mod format_conversions;

// use nu_engine::EvaluationContext;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(data.as_bytes(), 0, b"", b"", true);
    let (lite_block, err2) = nu_parser::lite_parse(&tokens);

    if err.is_none() && err2.is_none() {
        let context = create_default_context();
        {
            let mut working_set = StateWorkingSet::new(&context);
            working_set.add_file("quickcheck".into(), data.as_bytes());

            let _ = nu_parser::parse_block(&mut working_set, &lite_block, false, &[], false);
        }
    }
    true
}
