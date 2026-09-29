use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::prelude::*;

// Named type aliases (`struct Name = <shape>`)

#[test]
fn type_alias_in_signature() -> Result {
    test()
        .run("struct UserId = int; def bump [id: UserId] { $id + 1 }; bump 41")
        .expect_value_eq(42)
}

#[test]
fn type_alias_record_in_signature() -> Result {
    test()
        .run("struct Pt { x: int, y: int }; def f [p: Pt] { $p.x }; f {x: 3, y: 4}")
        .expect_value_eq(3)
}

#[test]
fn type_alias_rejects_wrong_input() -> Result {
    test()
        .run("struct UserId = int; def bump [id: UserId] { $id }; bump 'abc'")
        .expect_error_code_eq("nu::parser::parse_mismatch")
}

#[test]
fn type_alias_referencing_other_type() -> Result {
    test()
        .run("struct Pt { x: int, y: int }; struct Pair { a: Pt, b: Pt }; def f [p: Pair] { $p.b.y }; f {a: {x: 1, y: 2}, b: {x: 3, y: 4}}")
        .expect_value_eq(4)
}

#[test]
fn type_cannot_redefine_builtin() -> Result {
    test()
        .run("struct int = string")
        .expect_parse_error()
        .map(drop)
}

// Enum types (`enum Name { ... }`)

#[test]
fn enum_unit_variant() -> Result {
    test()
        .run("enum Shape { circle: float, point }; Shape.point | describe")
        .expect_value_eq("Shape")
}

#[test]
fn enum_payload_variant() -> Result {
    test()
        .run("enum Shape { circle: float, point }; Shape.circle 2.5 | describe")
        .expect_value_eq("Shape")
}

#[test]
fn enum_base_record() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; (Shape.circle {radius: 2.0}).radius")
        .expect_value_eq(2.0)
}

#[test]
fn enum_serializes_as_base_record() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; Shape.circle {radius: 2.0} | to json --raw")
        .expect_value_eq(r#"{"kind":"circle","radius":2.0}"#)
}

#[test]
fn enum_unknown_variant_is_parse_error() -> Result {
    let err = test()
        .run("enum Shape { circle, point }; Shape.cirle")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("unknown variant `cirle`"));
    Ok(())
}

#[test]
fn enum_duplicate_variant_is_error() -> Result {
    test()
        .run("enum Shape { a, a }")
        .expect_parse_error()
        .map(drop)
}

#[test]
fn enum_unit_variant_rejects_payload() -> Result {
    test()
        .run("enum Shape { point }; Shape.point 5")
        .expect_parse_error()
        .map(drop)
}

#[test]
fn enum_wrong_payload_type_is_error() -> Result {
    test()
        .run(r#"enum Shape { circle: float }; Shape.circle "nope""#)
        .expect_parse_error()
        .map(drop)
}

#[test]
fn enum_payload_checked_at_runtime() -> Result {
    let err = test()
        .run("enum Shape { circle: float }; let x: any = 'hi'; Shape.circle $x")
        .expect_shell_error()?;
    assert!(format!("{err:?}").contains("float"));
    Ok(())
}

#[test]
fn enum_in_signature_accepts_own_values() -> Result {
    test()
        .run("enum Shape { point }; def f [s: Shape] { $s | describe }; f Shape.point")
        .expect_value_eq("Shape")
}

#[test]
fn enum_base_record() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; (Shape.circle {radius: 2.0}).payload.radius")
        .expect_value_eq(2.0)
}

#[test]
fn enum_serializes_as_base_record() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; Shape.circle {radius: 2.0} | to json --raw")
        .expect_value_eq(r#"{"kind":"circle","payload":{"radius":2.0}}"#)
}

#[test]
fn enum_payload_field_kind_is_allowed() -> Result {
    // The payload nests under `payload`, so a record payload may use
    // `kind`/`payload` field names without colliding with the encoding.
    test()
        .run(r#"enum S { a: record<x: int, kind: string> }; S.a {x: 1, kind: "mine"} | to nuon --raw"#)
        .expect_value_eq(r#"{kind:a,payload:{x:1,kind:mine}}"#)
}

#[test]
fn enum_payload_field_payload_is_allowed() -> Result {
    test()
        .run(r#"enum S { a: record<payload: string> }; S.a {payload: "inner"} | to nuon --raw"#)
        .expect_value_eq(r#"{kind:a,payload:{payload:inner}}"#)
}

// Match destructuring + exhaustiveness

#[test]
fn enum_match_destructures_payload() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; match (Shape.circle {radius: 2.0}) { {kind: 'circle', payload: {radius: $r}} => { $r * $r }, {kind: 'point'} => 0.0 }")
        .expect_value_eq(4.0)
}

#[test]
fn enum_match_missing_variant_is_error() -> Result {
    let err = test()
        .run("enum Shape { a, b, c }; let s = Shape.a; match $s { {kind: 'a'} => 'a', {kind: 'b'} => 'b' }")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("missing variants: c"));
    Ok(())
}

#[test]
fn enum_match_wildcard_covers_all() -> Result {
    test()
        .run("enum Shape { a, b }; let s = Shape.b; match $s { {kind: 'a'} => 'a', _ => 'other' }")
        .expect_value_eq("other")
}

#[test]
fn enum_match_or_pattern_covers() -> Result {
    test()
        .run("enum Shape { a, b }; let s = Shape.b; match $s { {kind: 'a'} | {kind: 'b'} => 'ab' }")
        .expect_value_eq("ab")
}

#[test]
fn match_on_plain_record_unchanged() -> Result {
    test()
        .run("match {kind: 'x'} { {kind: 'x'} => 'yes' }")
        .expect_value_eq("yes")
}

// Expression-position constructors

#[test]
fn enum_constructor_in_list() -> Result {
    test()
        .run("enum S { a, b }; [S.a, S.b] | describe")
        .expect_value_eq("list<S>")
}

#[test]
fn enum_constructor_in_comparison() -> Result {
    test()
        .run("enum S { a }; let s = S.a; $s == S.a")
        .expect_value_eq(true)
}

#[test]
fn enum_constructor_missing_payload_is_error() -> Result {
    test()
        .run("enum S { a: int, b }; [S.a]")
        .expect_parse_error()
        .map(drop)
}

// Qualified variant patterns

#[test]
fn enum_match_qualified_variant() -> Result {
    test()
        .run("enum S { circle: record<radius: float>, point }; let c = S.point; match $c { S.circle {radius: $r} => $r, S.point => 99 }")
        .expect_value_eq(99)
}

#[test]
fn enum_match_qualified_variant_binds_payload() -> Result {
    test()
        .run("enum S { circle: record<radius: float>, point }; let c = S.circle {radius: 2.0}; match $c { S.circle {radius: $r} => { $r * $r }, S.point => 0.0 }")
        .expect_value_eq(4.0)
}

#[test]
fn enum_match_qualified_variant_binds_scalar_payload() -> Result {
    // A bare pattern after the variant binds the whole payload —
    // the trailing pattern always matches the payload, never the
    // `{kind, payload}` base record.
    test()
        .run("enum S { a: int, b }; let s = S.a 41; match $s { S.a $v => { $v + 1 }, S.b => 0 }")
        .expect_value_eq(42)
}

#[test]
fn enum_match_qualified_variant_ignores_payload() -> Result {
    test()
        .run("enum S { a: int, b }; let s = S.a 41; match $s { S.a _ => 'a', S.b => 'b' }")
        .expect_value_eq("a")
}

#[test]
fn enum_match_payload_pattern_on_unit_variant_is_error() -> Result {
    let err = test()
        .run("enum S { a: int, b }; match (S.a 1) { S.b $v => $v, S.a $v => $v }")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("unit variant"));
    Ok(())
}

#[test]
fn enum_match_record_payload_pattern_on_scalar_is_error() -> Result {
    // `a: int` can never satisfy a `{x: ...}` payload pattern —
    // an error beats a silently-never-matching arm.
    let err = test()
        .run("enum S { a: int, b }; match (S.a 1) { S.a {x: $f} => $f, S.b => 0 }")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("cannot match"));
    Ok(())
}

#[test]
fn enum_match_qualified_or_pattern() -> Result {
    test()
        .run("enum S { a, b }; let s = S.b; match $s { S.a | S.b => 'ab', _ => 'x' }")
        .expect_value_eq("ab")
}

#[test]
fn enum_match_qualified_unknown_variant() -> Result {
    test()
        .run("enum S { a, b }; let s = S.a; match $s { S.bogus => 1, _ => 2 }")
        .expect_parse_error()
        .map(drop)
}

#[test]
fn enum_match_qualified_still_checks_exhaustiveness() -> Result {
    let err = test()
        .run("enum S { a, b, c }; let s = S.a; match $s { S.a => 1, S.b => 2 }")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("missing variants: c"));
    Ok(())
}

#[test]
fn enum_match_exhaustive_output_is_not_union_nothing() -> Result {
    test()
        .run("enum S { a, b }; def f [s: S]: nothing -> int { match $s { S.a => 1, S.b => 2 } }; f (S.a)")
        .expect_value_eq(1)
}

// Qualified module names

#[test]
fn module_qualified_constructor() -> Result {
    test()
        .run("module m { export enum T { x, y }}; use m; m.T.x | describe")
        .expect_value_eq("T")
}

#[test]
fn module_qualified_type_in_signature() -> Result {
    test()
        .run("module m { export enum T { x, y }}; use m; def f [s: m.T] { $s | describe }; f (m.T.y)")
        .expect_value_eq("T")
}

#[test]
fn module_qualified_variant_pattern() -> Result {
    test()
        .run("module m { export enum T { x, y }}; use m; let s = m.T.y; match $s { m.T.x => 1, m.T.y => 2 }")
        .expect_value_eq(2)
}

#[test]
fn module_type_not_leaked_by_bare_use() -> Result {
    test()
        .run("module m { export enum T { x }}; use m; T.x")
        .expect_error_code_eq("nu::shell::external_command")
}

// from-record round-trip

#[test]
fn enum_from_record() -> Result {
    test()
        .run(r#"enum S { circle: record<radius: float>, point }; S.from-record {kind: "circle", payload: {radius: 2.5}} | describe"#)
        .expect_value_eq("S")
}

#[test]
fn enum_from_record_roundtrip_json() -> Result {
    test()
        .run("enum S { a: int, b }; let c = S.a 5 | to json --raw | from json | S.from-record $in; match $c { S.a $p => $p, S.b => 0 }")
        .expect_value_eq(5)
}

#[test]
fn enum_from_record_unknown_variant() -> Result {
    test()
        .run(r#"enum S { a }; S.from-record {kind: "zzz"}"#)
        .expect_shell_error()
        .map(drop)
}

#[test]
fn enum_from_record_validates_payload() -> Result {
    test()
        .run(r#"enum S { a: int, b }; S.from-record {kind: "a", payload: "nope"}"#)
        .expect_shell_error()
        .map(drop)
}

#[test]
fn enum_reserved_field_roundtrip() -> Result {
    // A record payload may use the `payload` field name — the encoding
    // nests it, so serialization and `from-record` round-trip cleanly.
    test()
        .run(r#"enum S { a: record<payload: string> }; let c = S.a {payload: "inner"} | to nuon --raw | from nuon | S.from-record $in; match $c { S.a {payload: $p} => $p }"#)
        .expect_value_eq("inner")
}

// Internal commands are hidden

#[test]
fn enum_construct_is_not_user_callable() -> Result {
    test()
        .run("enum S { a }; enum-construct S a")
        .expect_error_code_eq("nu::shell::external_command")
}

// mut + enforce-runtime-annotations

#[test]
fn enum_mut_reassign_same_enum_ok() -> Result {
    test()
        .run("enum S { a, b }; mut s = S.a; $s = S.b; $s | describe")
        .expect_value_eq("S")
}

#[test]
fn enum_mut_reassign_other_type_fails() -> Result {
    test()
        .run(r#"enum S { a, b }; mut s = S.a; $s = "str""#)
        .expect_parse_error()
        .map(drop)
}

// Modules

#[test]
fn module_export_type() -> Result {
    Playground::setup("module_export_type", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "shapes.nu",
            "
                export enum Shape { circle: record<radius: float>, point }
                export def area [s: Shape] {
                    match $s { {kind: 'circle', radius: $r} => { $r * $r }, {kind: 'point'} => 0.0 }
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("use shapes.nu *; let c = Shape.circle {radius: 2.0}; area $c")
            .expect_value_eq(4.0)
    })
}

#[test]
fn module_import_named_type() -> Result {
    Playground::setup("module_import_named_type", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "shapes.nu",
            "
                export enum Shape { circle: float, point }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("use shapes.nu Shape; Shape.point | describe")
            .expect_value_eq("Shape")
    })
}
