use nu_protocol::engine::StateWorkingSet;
use nu_test_support::{
    fs::Stub::{EmptyFile, FileWithContent},
    prelude::*,
};
use rstest::rstest;

#[test]
fn alias_names_a_command_output_and_a_parameter() -> Result {
    Playground::setup("type_alias_ls", |dirs, playground| {
        playground.with_files(&[EmptyFile("a.txt"), EmptyFile("b.txt")]);

        let code = "
            type LsType = table<name: string, type: string, size: filesize, modified: datetime>

            def my-ls []: nothing -> LsType {
              ls
            }

            def process-files [files: LsType] {
              $files | where size > 1mb
            }

            let big: LsType = my-ls | where size > 10mb
            [(my-ls | length) (process-files (my-ls) | length) ($big | length)]
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(vec![2, 0, 0])
    })
}

#[rstest]
#[case::positional("def f [x: N] { $x }; f 1", 1)]
#[case::flag("def f [--x: N] { $x }; f --x 1", 1)]
#[case::rest("def f [...x: N] { $x | math sum }; f 1 2", 3)]
#[case::closure_parameter("do {|x: N| $x } 1", 1)]
#[case::output_type("def f []: nothing -> N { 1 }; f", 1)]
#[case::let_annotation("let x: N = 1; $x", 1)]
#[case::mut_annotation("mut x: N = 1; $x += 1; $x", 2)]
#[case::const_annotation("const x: N = 1; $x", 1)]
#[case::inside_a_generic_type("let x: list<N> = [1]; $x.0", 1)]
#[case::another_alias("type L = list<N>; let x: L = [1]; $x.0", 1)]
fn alias_is_accepted_where_its_type_is(#[case] code: &str, #[case] expected: i64) -> Result {
    test()
        .run(format!("type N = int; {code}"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::positional("def f [x: N] { $x }; f a", "nu::parser::parse_mismatch")]
#[case::flag("def f [--x: N] { $x }; f --x a", "nu::parser::parse_mismatch")]
#[case::rest("def f [...x: N] { $x }; f 1 a", "nu::parser::parse_mismatch")]
#[case::closure_parameter("do {|x: N| $x } a", "nu::shell::cant_convert")]
#[case::output_type("def f []: nothing -> N { 'a' }", "nu::parser::output_type_mismatch")]
#[case::let_annotation("let x: N = 'a'", "nu::parser::type_mismatch")]
#[case::inside_a_generic_type("let x: list<N> = ['a']", "nu::parser::type_mismatch")]
#[case::another_alias("type L = list<N>; let x: L = ['a']", "nu::parser::type_mismatch")]
fn alias_rejects_what_its_type_rejects(#[case] code: &str, #[case] error: &str) -> Result {
    test()
        .run(format!("type N = int; {code}"))
        .expect_error_code_eq(error)
}

#[test]
fn alias_is_erased_after_parsing() -> Result {
    let code = "
        type Point = record<x: int, y: int>
        def f [p: Point]: nothing -> Point { $p }
        let p: Point = {x: 1, y: 2}
        [($p | describe) (scope commands | where name == f | get signatures.0 | values | flatten | get syntax_shape.1)]
    ";
    test()
        .run(code)
        .expect_value_eq(["record<x: int, y: int>", "record<x: int, y: int>"])
}

#[test]
fn alias_of_directory_keeps_directory_parameter_behavior() -> Result {
    let code = "type Dir = directory; def f [d: Dir] { $d }; (f ~) == ('~' | path expand)";
    test().run(code).expect_value_eq(true)
}

#[test]
fn alias_of_a_union_type() -> Result {
    let code = r#"
        type Id = oneof<int, string>
        type MaybeId = oneof<Id, nothing>
        def lookup [id: MaybeId] { if $id == null { "everything" } else { $"item ($id)" } }
        [(lookup null) (lookup 3) (lookup x)]
    "#;
    test()
        .run(code)
        .expect_value_eq(["everything", "item 3", "item x"])
}

#[rstest]
#[case::table_row("tableRow")]
#[case::listing("listing")]
#[case::records("records")]
#[case::oneofs("oneofs")]
fn alias_name_can_start_with_a_generic_type_name(#[case] name: &str) -> Result {
    let code = format!("type {name} = int; def f [x: {name}] {{ $x }}; f 7");
    test().run(code).expect_value_eq(7)
}

#[rstest]
#[case::used_before_definition(
    "def f [x: Size] {}; type Size = filesize",
    "nu::parser::type_alias_used_before_definition"
)]
#[case::used_before_definition_in_inner_block(
    "do { let x: Size = 1kb }; type Size = filesize",
    "nu::parser::type_alias_used_before_definition"
)]
#[case::self_reference(
    "type Tree = record<children: list<Tree>>",
    "nu::parser::type_alias_self_reference"
)]
#[case::shadows_builtin("type int = string", "nu::parser::type_alias_shadows_builtin")]
#[case::shadows_generic_builtin("type list = string", "nu::parser::type_alias_shadows_builtin")]
#[case::name_with_at("type my@type = int", "nu::parser::type_alias_name_not_valid")]
#[case::name_with_digit_first("type 1x = int", "nu::parser::type_alias_name_not_valid")]
#[case::name_with_colon("type a:b = int", "nu::parser::type_alias_name_not_valid")]
#[case::with_params(
    "type Foo = int; def f [x: list<Foo<int>>] {}",
    "nu::parser::type_alias_with_params"
)]
#[case::missing_sign("type Foo int", "nu::parser::type_alias_missing_equals")]
#[case::name_only("type Foo", "nu::parser::type_alias_missing_equals")]
#[case::missing_name("type = int", "nu::parser::assignment_mismatch")]
#[case::no_spaces_around_sign("type Foo=int", "nu::parser::incorrect_value")]
#[case::missing_type("type Foo =", "nu::parser::missing_type")]
#[case::extra_tokens("type Foo = int string", "nu::parser::extra_tokens")]
#[case::unknown_type_on_the_right("type Foo = vec<int>", "nu::parser::unknown_type")]
#[case::unknown_alias("def f [x: Nope] {}", "nu::parser::unknown_type")]
fn type_alias_errors(#[case] code: &str, #[case] error: &str) -> Result {
    test().run(code).expect_error_code_eq(error)
}

#[rstest]
#[case::bare_type("type ls", Some("which ls"))]
#[case::bare_type_with_arguments("type ls -a", Some("which ls"))]
#[case::export_type("module m { export type Foo }", None)]
fn missing_equals_help_shows_the_alias_form(
    #[case] code: &str,
    #[case] lookup_hint: Option<&str>,
) -> Result {
    let err = test().run(code).expect_parse_error()?;
    let ParseError::TypeAliasMissingEquals { help, .. } = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_contains("= <type>`", &help);
    match lookup_hint {
        Some(hint) => assert_contains(hint, &help),
        None => assert_contains_not("which", &help),
    }
    Ok(())
}

#[test]
fn completer_on_the_right_is_an_error() -> Result {
    let err = test()
        .run("type Color = string@[red green blue]")
        .expect_parse_error()?;
    assert!(
        matches!(&err, ParseError::LabeledError(msg, ..) if msg == "Unexpected custom completer in type spec"),
        "unexpected error: {err:?}"
    );
    Ok(())
}

#[test]
fn broken_alias_is_reported_once_and_defined_as_any() {
    let engine_state = nu_cmd_lang::create_default_context();
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(
        &mut working_set,
        None,
        b"type Foo = vec<int>; def f [x: Foo] { $x }; f 1",
        false,
    );
    assert!(
        matches!(working_set.parse_errors.as_slice(), [ParseError::UnknownType(span)] if span.len() == "vec<int>".len()),
        "expected one UnknownType at the definition, got {:?}",
        working_set.parse_errors
    );
}

#[test]
fn alias_refers_to_an_earlier_definition_of_its_name() -> Result {
    let code = "type Tree = int; type Tree = list<Tree>; let t: Tree = [1]; $t.0";
    test().run(code).expect_value_eq(1)
}

#[test]
fn redefinition_applies_from_that_statement_on() -> Result {
    let code = "
        type Id = int
        def by-number [id: Id] { $id | describe }
        type Id = string
        def by-name [id: Id] { $id | describe }
        [(by-number 1) (by-name x)]
    ";
    test().run(code).expect_value_eq(["int", "string"])
}

#[test]
fn alias_is_local_to_its_block() -> Result {
    let code = "
        def report [] {
          type Row = record<name: string, total: int>
          let rows: list<Row> = [{name: apples, total: 3}]
          $rows
        }
        let r: Row = {name: pears, total: 1}
    ";
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[test]
fn alias_persists_across_repl_lines() -> Result {
    test()
        .run_multiple([
            "type Point = record<x: int, y: int>",
            "def f [p: Point] { $p.x }",
            "f {x: 4, y: 5}",
        ])
        .expect_value_eq(4)
}

#[test]
fn alias_comes_in_with_source() -> Result {
    Playground::setup("type_alias_source", |dirs, playground| {
        playground.with_files(&[FileWithContent(
            "types.nu",
            "type Point = record<x: int, y: int>",
        )]);

        test()
            .cwd(dirs.test())
            .run("source types.nu; let p: Point = {x: 1, y: 2}; $p.y")
            .expect_value_eq(2)
    })
}

#[test]
fn call_above_its_def_is_checked_at_run_time() -> Result {
    // The predeclared signature is parsed before the `type` statement runs, so the parser sees
    // `any` for `size`; the run-time check against the final signature catches the string.
    test()
        .run("type Size = filesize; f 'big'; def f [size: Size] { $size }")
        .expect_error_code_eq("nu::shell::cant_convert")
}

#[rstest]
#[case::def("def type [] { 1 }", "nu::parser::name_is_keyword")]
#[case::extern_("extern \"type\" [...args]", "nu::parser::name_is_keyword")]
#[case::alias("alias type = ls", "nu::parser::name_is_keyword")]
#[case::def_export_type("def 'export type' [] { 1 }", "nu::parser::name_is_keyword")]
#[case::exported_from_a_module(
    "module m { export def type [] { 1 } }",
    "nu::parser::name_is_keyword"
)]
#[case::module_main(
    "module type { export def main [] { 1 } }; use type",
    "nu::parser::keyword_shadow_module_main"
)]
fn type_is_a_reserved_name(#[case] code: &str, #[case] error: &str) -> Result {
    test().run(code).expect_error_code_eq(error)
}

#[test]
fn cant_alias_the_type_keyword() -> Result {
    test()
        .run("alias ty = type")
        .expect_error_code_eq("nu::parser::cant_alias_keyword")
}

#[test]
fn type_is_not_allowed_in_a_pipeline() -> Result {
    test()
        .run("ls | type X = int")
        .expect_error_code_eq("nu::parser::unexpected_keyword")
}

#[test]
fn type_help() -> Result {
    let output: String = test().run("type --help")?;
    assert_contains("Give a name to a type annotation.", &output);
    Ok(())
}

#[test]
fn help_shows_the_expanded_type() -> Result {
    let output: String = test().run(
        "type Point = record<x: int, y: int>; def f []: nothing -> Point { {x: 1, y: 2} }; help f",
    )?;
    assert_contains("record<x: int, y: int>", &output);
    Ok(())
}

#[test]
fn parameter_of_an_aliased_type_takes_a_completer() -> Result {
    let code = "
        def animals [] { [cat dog eel] }
        type Animal = string
        def f [a: Animal@animals, b: Animal, --flag: Animal@[lion]] {}
        scope commands | where name == f | get signatures.0 | values | flatten
        | where parameter_name in [a b flag]
        | select parameter_name syntax_shape completion | sort-by parameter_name | to nuon
    ";
    test().run(code).expect_value_eq(
        "[[parameter_name, syntax_shape, completion]; [a, string, animals], [b, string, null], [flag, string, [lion]]]",
    )
}

const ZOO: &str = "
    module zoo {
      export type Animal = string
      export def feed [animal: Animal] { $\"feeding the ($animal)\" }
      export module keepers {
        export type Keeper = string
      }
    }
";

#[rstest]
#[case::by_name("use zoo Animal", "let a: Animal = 'cat'; $a", "cat")]
#[case::by_list(
    "use zoo [Animal feed]",
    "def adopt [a: Animal] { feed $a }; adopt cat",
    "feeding the cat"
)]
#[case::glob("use zoo *", "let a: Animal = 'cat'; $a", "cat")]
#[case::submodule_by_name("use zoo keepers Keeper", "let k: Keeper = 'kim'; $k", "kim")]
#[case::overlay("overlay use zoo", "let a: Animal = 'cat'; $a", "cat")]
fn alias_is_imported(#[case] import: &str, #[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{ZOO}\n{import}\n{code}"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::bare("use zoo", "let a: Animal = 'cat'")]
#[case::bare_then_command("use zoo", "zoo feed cat; let a: Animal = 'cat'")]
#[case::glob_skips_submodule_aliases("use zoo *", "let k: Keeper = 'kim'")]
#[case::prefixed_overlay("overlay use --prefix zoo", "let a: Animal = 'cat'")]
#[case::private(
    "module m { type P = int; export def f [] {} }; use m *",
    "let p: P = 1"
)]
fn alias_is_not_imported(#[case] import: &str, #[case] code: &str) -> Result {
    test()
        .run(format!("{ZOO}\n{import}\n{code}"))
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[rstest]
#[case::by_name("type Animal = string; hide Animal")]
#[case::module_member("use zoo Animal; hide zoo Animal")]
#[case::module_member_list("use zoo [Animal feed]; hide zoo [Animal]")]
#[case::module_glob("use zoo *; hide zoo *")]
fn hidden_alias_is_unknown(#[case] code: &str) -> Result {
    test()
        .run(format!("{ZOO}\n{code}\nlet a: Animal = 'cat'"))
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[test]
fn hiding_a_module_member_alias_keeps_the_module_commands() -> Result {
    test()
        .run(format!(
            "{ZOO}\nuse zoo [Animal feed]\nhide zoo Animal\nfeed cat"
        ))
        .expect_value_eq("feeding the cat")
}

#[test]
fn hidden_alias_stays_hidden_on_later_repl_lines() -> Result {
    let mut tester = test();
    let () = tester.run("type Id = int")?;
    let () = tester.run("hide Id")?;
    tester
        .run("let x: Id = 1")
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[rstest]
#[case::imported_again(
    &format!("{ZOO}\nuse zoo Animal\nhide Animal\nuse zoo Animal\nlet a: Animal = 'cat'; $a"),
    test_value!("cat")
)]
#[case::defined_again("type Id = int; hide Id; type Id = string; let x: Id = 'a'; $x", test_value!("a"))]
#[case::command_of_the_same_name_hidden_first(
    "def Id [] { 'cmd' }; type Id = int; hide Id; let x: Id = 5; $x",
    test_value!(5)
)]
#[case::unknown_name("hide Nope; 'ok'", test_value!("ok"))]
fn hide_and_type_aliases(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn hiding_a_missing_module_member_is_still_an_error() -> Result {
    test()
        .run(format!("{ZOO}\nhide zoo Nope"))
        .expect_error_code_eq("nu::parser::export_not_found")
}

#[test]
fn missing_export_is_still_reported() -> Result {
    test()
        .run(format!("{ZOO}\nuse zoo [Animal Nope]"))
        .expect_error_code_eq("nu::parser::export_not_found")
}

#[test]
fn command_and_alias_with_the_same_name_import_together() -> Result {
    let code = "
        module m {
          export type Animal = string
          export def Animal [] { 'cmd' }
        }
        use m Animal
        let a: Animal = 'cat'
        [$a (Animal)]
    ";
    test().run(code).expect_value_eq(["cat", "cmd"])
}

#[test]
fn alias_is_reexported() -> Result {
    let code = "
        module geo {
          export type Point = record<x: float, y: float>
        }
        module shapes {
          export use geo Point
          export type Circle = record<center: Point, radius: float>
        }
        use shapes [Point Circle]
        let c: Circle = {center: {x: 0.0, y: 0.0}, radius: 1.0}
        let p: Point = $c.center
        $p.x
    ";
    test().run(code).expect_value_eq(0.0)
}

#[test]
fn alias_is_kept_by_overlay_hide_keep_custom() -> Result {
    let code = "overlay new o; type T = int; overlay hide o --keep-custom; let x: T = 1; $x";
    test().run(code).expect_value_eq(1)
}

#[test]
fn alias_goes_away_with_its_overlay() -> Result {
    let code = "overlay new o; type T = int; overlay hide o; let x: T = 1";
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[test]
fn alias_comes_from_a_module_file() -> Result {
    Playground::setup("type_alias_module_file", |dirs, playground| {
        playground.with_files(&[FileWithContent(
            "geo.nu",
            "
            export type Point = record<x: float, y: float>
            export type Polygon = list<Point>

            # Area of a simple polygon, by the shoelace formula.
            export def area [poly: Polygon]: nothing -> float {
              let n = $poly | length
              0..<$n | each {|i|
                let a = $poly | get $i
                let b = $poly | get (($i + 1) mod $n)
                $a.x * $b.y - $b.x * $a.y
              } | math sum | math abs | $in / 2
            }
            ",
        )]);

        let code = "
            use geo.nu [Polygon area]
            let square: Polygon = [{x: 0.0, y: 0.0} {x: 2.0, y: 0.0} {x: 2.0, y: 2.0} {x: 0.0, y: 2.0}]
            area $square
        ";
        test().cwd(dirs.test()).run(code).expect_value_eq(4.0)
    })
}
