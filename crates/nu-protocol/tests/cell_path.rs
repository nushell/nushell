use nu_protocol::ast::CellPath;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("abc")]
fn engine_eq_from_str(#[case] input: &str) -> Result {
    let mut tester = test();
    let () = tester.run("def cell-path [cp: cell-path] { $cp }")?;
    let via_engine: CellPath = tester.run(format!("cell-path {input}"))?;
    let via_from_str: CellPath = input.parse().unwrap();
    assert_eq!(via_engine, via_from_str);
    Ok(())
}
