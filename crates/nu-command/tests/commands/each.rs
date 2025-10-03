use nu_test_support::nu;

#[test]
fn each_works_separately() {
    let actual = nu!("echo [1 2 3] | each { |it| echo $it 10 | math sum } | to json -r");

    assert_eq!(actual.out, "[11,12,13]");
}

#[test]
fn each_group_works() {
    let actual = nu!("echo [1 2 3 4 5 6] | chunks 3 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[4,5,6]]");
}

#[test]
fn each_window() {
    let actual = nu!("echo [1 2 3 4] | window 3 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[2,3,4]]");
}

#[test]
fn each_window_stride() {
    let actual = nu!("echo [1 2 3 4 5 6] | window 3 -s 2 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[3,4,5]]");
}

#[test]
fn each_no_args_in_block() {
    let actual = nu!("echo [[foo bar]; [a b] [c d] [e f]] | each {|i| $i | to json -r } | get 1");

    assert_eq!(actual.out, r#"{"foo":"c","bar":"d"}"#);
}

#[test]
fn each_implicit_it_in_block() {
    let actual = nu!(
        "echo [[foo bar]; [a b] [c d] [e f]] | each { |it| nu --testbin cococo $it.foo } | str join"
    );

    assert_eq!(actual.out, "ace");
}

#[test]
fn each_uses_enumerate_index() {
    let actual = nu!("[7 8 9 10] | enumerate | each {|el| $el.index } | to nuon");

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn each_while_uses_enumerate_index() {
    let actual = nu!("[7 8 9 10] | enumerate | each while {|el| $el.index } | to nuon");

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn errors_in_nested_each_show() {
    let actual = nu!("[[1,2]] | each {|x| $x | each {|y| error make {msg: \"oh noes\"} } }");
    assert!(actual.err.contains("oh noes"))
}

#[test]
fn errors_in_nested_each_full_chain() {
    let actual = nu!(r#" 0..1 | each {|i| 0..1 | each {|j| error make {msg: boom} } } "#);

    let eval_block_with_input_count = actual.err.matches("eval_block_with_input").count();
    assert_eq!(eval_block_with_input_count, 2);
}

#[test]
fn each_noop_on_single_null() {
    let actual = nu!("null | each { \"test\" } | describe");

    assert_eq!(actual.out, "nothing");
}

#[test]
fn each_flatten_dont_collect() {
    let collected = nu!(r##"
        def round  [] { each {|e| print -n $"\(($e)\)"; $e } }
        def square [] { each {|e| print -n  $"[($e)]";  $e } }
        [0 3] | each {|e| $e..<($e + 3) | round } | flatten | square | ignore
    "##);

    assert_eq!(collected.out, r#"(0)(1)(2)[0][1][2](3)(4)(5)[3][4][5]"#);

    let streamed = nu!(r##"
        def round  [] { each {|e| print -n $"\(($e)\)"; $e } }
        def square [] { each {|e| print -n  $"[($e)]";  $e } }
        [0 3] | each --flatten {|e| $e..<($e + 3) | round } | square | ignore
    "##);

    assert_eq!(streamed.out, r#"(0)[0](1)[1](2)[2](3)[3](4)[4](5)[5]"#);
}
