use nu_test_support::prelude::*;

#[test]
fn chunk_by_on_empty_input_returns_empty_list() -> Result {
    test()
        .run("[] | chunk-by {|it| $it} | to nuon")
        .expect_value_eq("[]")
}

#[rustfmt::skip]
#[test]
fn chunk_by_strings_works() -> Result {
    let code = "
        [a a a b b b c c c a a a]
        | chunk-by {|it| $it}
    ";

    test().run(code).expect_value_eq([
        ["a", "a", "a"],
        ["b", "b", "b"],
        ["c", "c", "c"],
        ["a", "a", "a"],
    ])
}

#[test]
fn chunk_by_field_works() -> Result {
    #[derive(Debug, IntoValue)]
    struct Sample {
        name: &'static str,
        age: u32,
        cool: bool,
    }

    let data = [
        Sample {
            name: "bob",
            age: 20,
            cool: false,
        },
        Sample {
            name: "jane",
            age: 30,
            cool: false,
        },
        Sample {
            name: "marie",
            age: 19,
            cool: true,
        },
        Sample {
            name: "carl",
            age: 36,
            cool: true,
        },
    ];

    let code = "
        $in
        | chunk-by {|it| $it.cool}
        | length
    ";

    test().run_with_data(code, data).expect_value_eq(2)
}
