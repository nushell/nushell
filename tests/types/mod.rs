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
fn enum_duplicate_variant_is_error() -> Result {
    test().run("enum E { a, a }").expect_parse_error().map(drop)
}

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

fn enum_unknown_variant_is_parse_error() -> Result {
    let err = test()
        .run("enum Shape { circle, point }; Shape.cirle")
        .expect_parse_error()?;
    assert!(format!("{err:?}").contains("unknown variant `cirle`"));
    Ok(())
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

// Match destructuring

#[test]
fn enum_match_destructures_payload() -> Result {
    test()
        .run("enum Shape { circle: record<radius: float>, point }; match (Shape.circle {radius: 2.0}) { {kind: 'circle', payload: {radius: $r}} => { $r * $r }, {kind: 'point'} => 0.0 }")
        .expect_value_eq(4.0)
}

#[test]
fn match_on_plain_record_unchanged() -> Result {
    test()
        .run("match {x: 1, y: 2} { {x: $a, y: $b} => { $a + $b }, _ => 0 }")
        .expect_value_eq(3)
}

// Modules

#[test]
fn module_export_type() -> Result {
    let lines = [
        "module shapes {",
        "    export struct Pt { x: int, y: int }",
        "    export def origin []: nothing -> Pt { {x: 0, y: 0} }",
        "}",
        "use shapes",
        "shapes origin | describe",
    ];
    let script = lines.join("\n");
    test()
        .run(&script)
        .expect_value_eq("record<x: int, y: int>")
}

#[test]
fn module_import_named_type() -> Result {
    Playground::setup("module_import_named_type", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "shapes.nu",
            "
                export struct Pt { x: int, y: int }
                export def make [x: int, y: int]: nothing -> Pt { {x: $x, y: $y} }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("use shapes.nu [Pt, make]; make 3 4 | describe")
            .expect_value_eq("record<x: int, y: int>")
    })
}
