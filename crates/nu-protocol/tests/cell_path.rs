use nu_protocol::ast::CellPath;
use nu_test_support::{prelude::*, test_cell_path};
use rstest::rstest;

#[rstest]
#[case("'quoted member'?!.name")]
#[case("'quoted member'.name")]
#[case("`two words`!?.name")]
#[case("`two words`.name")]
#[case("$.")]
#[case("$.")]
#[case("$.34")]
#[case("$.abc")]
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
#[case(test_cell_path!("double quoted"!?.name))]
#[case(test_cell_path!("double quoted".name))]
#[case(test_cell_path!("quoted member"?!.name))]
#[case(test_cell_path!("quoted member".name))]
#[case(test_cell_path!("two words"!?.name))]
#[case(test_cell_path!("two words".name))]
#[case(test_cell_path!(0!?))]
#[case(test_cell_path!(0!))]
#[case(test_cell_path!(0?!))]
#[case(test_cell_path!(0?))]
#[case(test_cell_path!(0.abc))]
#[case(test_cell_path!(0))]
#[case(test_cell_path!(34))]
#[case(test_cell_path!(abc!?.def))]
#[case(test_cell_path!(abc!.def))]
#[case(test_cell_path!(abc?!.def))]
#[case(test_cell_path!(abc.0?.def?))]
#[case(test_cell_path!(abc.0))]
#[case(test_cell_path!(abc))]
#[case(test_cell_path!(abc))]
#[case(test_cell_path!(abc))]
#[case(test_cell_path!(items.0 .1))]
#[case(test_cell_path!(items.0b1010))]
#[case(test_cell_path!(items.0o12))]
#[case(test_cell_path!(items.1_000))]
#[case(CellPath::empty())]
fn roundtrip(#[case] input: CellPath) {
    let serialized = serde_json::to_string(&input).unwrap();
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(input, deserialized);
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

#[rstest]
// `PathMember`'s `Display` escapes a NUL as `\0`. The reader used to have no arm for that
// escape, and `alt` fell through to the bare-character parser, which read the backslash and
// the `0` as two ordinary characters — so the path parsed fine and came back as the four
// characters `a\0b`.
#[case(r#"$."a\0b""#)]
#[case(r#"$."a\0b"."c\0d""#)]
#[case(r#"$."""#)]
#[case(r#"$."\0""#)]
fn a_nul_in_a_member_round_trips(#[case] input: &str) {
    let parsed: CellPath = input.parse().expect("cell path parses");
    assert_eq!(
        parsed.to_string(),
        input,
        "the path must serialize back to the text it was parsed from"
    );
}

/// `in` and `not-in` between cell paths look for the left path's members as a contiguous run in
/// the right path. The paths come in as data, so the type checker sees `any` and lets the
/// operators through to the runtime.
#[rstest]
#[case::same_path(test_cell_path!(a), test_cell_path!(a), true)]
#[case::other_member(test_cell_path!(x), test_cell_path!(a), false)]
#[case::inner_run(test_cell_path!(b.c), test_cell_path!(a.b.c.d), true)]
#[case::int_member(test_cell_path!(0), test_cell_path!(items.0), true)]
#[case::not_contiguous(test_cell_path!(a.c), test_cell_path!(a.b.c), false)]
#[case::longer_than_rhs(test_cell_path!(a.b), test_cell_path!(a), false)]
#[case::empty_lhs(CellPath::empty(), test_cell_path!(a), true)]
fn cell_path_in_cell_path(
    #[case] lhs: CellPath,
    #[case] rhs: CellPath,
    #[case] expected: bool,
) -> Result {
    test()
        .run_with_data(
            "let paths = $in; [($paths.0 in $paths.1) ($paths.0 not-in $paths.1)]",
            [lhs, rhs],
        )
        .expect_value_eq([expected, !expected])
}
