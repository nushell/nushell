use nu_test_support::prelude::*;

#[test]
#[ignore = "incorrect test"]
fn md_empty() -> Result {
    let code = "echo [[]; []] | from json | to md";
    test().run(code).expect_value_eq("")
}

#[test]
fn md_empty_pretty() -> Result {
    let code = r#"echo "{}" | from json | to md -p"#;
    test().run(code).expect_value_eq("")
}

#[test]
fn md_simple() -> Result {
    let code = "echo 3 | to md";
    test().run(code).expect_value_eq("* 3")
}

#[test]
fn md_simple_pretty() -> Result {
    let code = "echo 3 | to md -p";
    test().run(code).expect_value_eq("* 3")
}

#[test]
fn md_table() -> Result {
    let code = "echo [[name]; [jason]] | to md";
    test()
        .run(code)
        .expect_value_eq("| name |\n| --- |\n| jason |")
}

#[test]
fn md_table_pretty() -> Result {
    let code = "echo [[name]; [joseph]] | to md -p";
    test()
        .run(code)
        .expect_value_eq("| name   |\n| ------ |\n| joseph |")
}

#[test]
fn md_combined() -> Result {
    let code = r#"
        def title [] {
            echo [[H1]; ["Nu top meals"]]
        };
    
        def meals [] {
            echo [[dish]; [Arepa] [Taco] [Pizza]]
        };
    
        title
        | append (meals)
        | to md --per-element --pretty
    "#;

    test()
        .run(code)
        .expect_value_eq("# Nu top meals\n| dish  |\n| ----- |\n| Arepa |\n| Taco  |\n| Pizza |")
}
