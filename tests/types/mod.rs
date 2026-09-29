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
