use nu_protocol::{engine::StateWorkingSet, Span};
use quickcheck_macros::quickcheck;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(data.as_bytes(), 0, b"", b"", true);

    if err.is_none() {
        let context = crate::create_default_context();
        {
            let mut working_set = StateWorkingSet::new(&context);
            let _ = working_set.add_file("quickcheck".into(), data.as_bytes());

            let _ =
                nu_parser::parse_block(&mut working_set, &tokens, Span::new(0, 0), false, false);
        }
    }
    true
}
