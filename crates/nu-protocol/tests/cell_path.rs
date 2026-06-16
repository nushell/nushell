use nu_protocol::ast::CellPath;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("'quoted member'?!.name")]
#[case("'quoted member'.name")]
#[case("`two words`!?.name")]
#[case("`two words`.name")]
#[case("$.")]
#[case("$.abc")]
#[case("0!?")]
#[case("0!")]
#[case("0?!")]
#[case("0?")]
#[case("0.abc")]
#[case("0")]
#[case("abc!?.def")]
#[case("abc!.def")]
#[case("abc?!.def")]
#[case("abc.0?.def?")]
#[case("abc.0")]
#[case("abc")]
#[case("items.0.1")]
#[case("items.0b1010")]
#[case("items.0o12")]
#[case("items.1_000")]
#[case(r#""double quoted"!?.name"#)]
#[case(r#""double quoted".name"#)]
fn engine_eq_from_str(#[case] input: &str) -> Result {
    let mut tester = test();
    let () = tester.run("def cell-path [cp: cell-path] { $cp }")?;
    let via_engine: CellPath = tester.run(format!("cell-path {input}"))?;
    let via_from_str: CellPath = input.parse().expect("cell path parses");
    assert_eq!(via_engine, via_from_str);
    Ok(())
}

#[rstest]
#[case("abc??")]
#[case("abc!!")]
#[case("abc?!?")]
#[case("abc!?!")]
#[case("0??")]
#[case("0!!")]
#[case("0?!?")]
#[case("0!?!")]
fn rejects_invalid_cell_paths(#[case] input: &str) {
    assert!(input.parse::<CellPath>().is_err());
}
