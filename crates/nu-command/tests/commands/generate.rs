use nu_protocol::test_value;
use nu_test_support::prelude::*;

#[test]
fn generate_no_next_break() -> Result {
    let code = "
        generate {|x|
            if $x == 3 {
                {out: $x}
            } else {
                {out: $x, next: ( $x + 1 )}
            }
        } 1
    ";
    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn generate_null_break() -> Result {
    let code = "
        generate {|x|
            if $x <= 3 {
                {out: $x, next: ($x + 1)}
            }
        } 1
    ";
    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn generate_allows_empty_output() -> Result {
    let code = "
        generate {|x|
            if $x == 1 {
                {next: ($x + 1)}
            } else if $x < 3 {
                {out: $x, next: ($x + 1)}
            }
        } 0
    ";
    test().run(code).expect_value_eq([0, 2])
}

#[test]
fn generate_allows_no_output() -> Result {
    let code = "
        generate {|x|
            if $x < 3 {
                {next: ($x + 1)}
            }
        } 0
    ";
    test().run(code).expect_value_eq([(); 0])
}

#[test]
fn generate_allows_null_state() -> Result {
    let code = r#"
        generate {|x|
            if $x == null {
                {out: "done"}
            } else if $x < 1 {
                {out: "going", next: ($x + 1)}
            } else {
                {out: "stopping", next: null}
            }
        } 0
    "#;
    test()
        .run(code)
        .expect_value_eq(["going", "stopping", "done"])
}

#[test]
fn generate_allows_null_output() -> Result {
    let code = r#"
        generate {|x|
            if $x == 3 {
                {out: "done"}
            } else {
                {out: null, next: ($x + 1)}
            }
        } 0
    "#;
    test().run(code).expect_value_eq(((), (), (), "done"))
}

#[test]
fn generate_disallows_extra_keys() -> Result {
    let err = test()
        .run("generate {|x| {foo: bar, out: $x}} 0 ")
        .expect_shell_error()?
        .generic_error()?;
    assert_eq!(err, "Invalid block return");
    Ok(())
}

#[test]
fn generate_disallows_list() -> Result {
    let err = test()
        .run("generate {|x| [$x, ($x + 1)]} 0 ")
        .expect_shell_error()?
        .generic_error()?;
    assert_eq!(err, "Invalid block return");
    Ok(())
}

#[test]
fn generate_disallows_primitive() -> Result {
    let err = test()
        .run("generate {|x| 1} 0")
        .expect_shell_error()?
        .generic_error()?;
    assert_eq!(err, "Invalid block return");
    Ok(())
}

#[test]
fn generate_allow_default_parameter() -> Result {
    let code = r#"
        generate {|x = 0|
            if $x == 3 {
                {out: "done"}
            } else {
                {out: null, next: ($x + 1)}
            }
        }
    "#;
    test().run(code).expect_value_eq(((), (), (), "done"))?;

    // if initial is given, use initial value
    let code = r#"
        generate {|x = 0|
            if $x == 3 {
                {out: "done"}
            } else {
                {out: null, next: ($x + 1)}
            }
        } 1
    "#;
    test().run(code).expect_value_eq(((), (), "done"))
}

#[test]
fn generate_raise_error_on_no_default_parameter_closure_and_init_val() -> Result {
    let code = r#"
        generate {|x|
            if $x == 3 {
                {out: "done"}
            } else {
                {out: null, next: ($x + 1)}
            }
        }
    "#;
    let err = test().run(code).expect_shell_error()?.generic_error()?;
    assert_eq!(err, "The initial value is missing");

    Ok(())
}

#[test]
fn generate_allows_pipeline_input() -> Result {
    test()
        .run("[1 2 3] | generate {|e, x=null| {out: $e, next: null}}")
        .expect_value_eq([1, 2, 3])
}

#[test]
fn generate_with_input_is_streaming() -> Result {
    let code = "
        let id = job id

        1..10
        | each {|x|
            $x | job send $id
            $x
        }
        | generate {|e, sum=0|
            let sum = $e + $sum
            {out: $sum, next: $sum}
        }
        | first 5
        | let sum

        let sent = generate {|_ = null|
            try { {next: null, out: (job recv --timeout 0sec)} }
        }

        {sum: $sum, sent: $sent}
    ";
    test().run(code).expect_value_eq(test_value!({
        sum: [1, 3, 6, 10, 15],
        sent: [1, 2, 3, 4, 5],
    }))
}
