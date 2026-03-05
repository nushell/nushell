use nu_test_support::prelude::*;

#[test]
#[ignore = "incorrect test"]
fn md_empty() -> Result {
    let code = "echo [[]; []] | from json | to md";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn md_empty_pretty() -> Result {
    let code = r#"echo "{}" | from json | to md -p"#;
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn md_simple() -> Result {
    let code = "echo 3 | to md";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "* 3");
    Ok(())
}

#[test]
fn md_simple_pretty() -> Result {
    let code = "echo 3 | to md -p";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "* 3");
    Ok(())
}

#[test]
fn md_table() -> Result {
    let code = "echo [[name]; [jason]] | to md";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "| name |\n| --- |\n| jason |");
    Ok(())
}

#[test]
fn md_table_pretty() -> Result {
    let code = "echo [[name]; [joseph]] | to md -p";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "| name   |\n| ------ |\n| joseph |");
    Ok(())
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

    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        "# Nu top meals\n| dish  |\n| ----- |\n| Arepa |\n| Taco  |\n| Pizza |"
    );
    Ok(())
}
