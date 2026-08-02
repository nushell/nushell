use nu_protocol::ast::CellPath;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case(test_cell_path!(To.Address), "to@example.com")]
#[case(test_cell_path!(To.Name), ())]
#[case(test_cell_path!("Reply-To".Address), "replyto@example.com")]
#[case(test_cell_path!("Reply-To".Name), "replyto@example.com")]
#[case(test_cell_path!(Subject), "Test Message")]
#[case(test_cell_path!("MIME-Version"), "1.0")]
#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn from_eml_get_to_field(#[case] cell_path: CellPath, #[case] expect: impl IntoValue) -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run_with_data("let cp = $in; open sample.eml | get $cp", cell_path)
        .expect_value_eq(expect)
}
