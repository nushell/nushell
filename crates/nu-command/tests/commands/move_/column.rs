use nu_test_support::prelude::*;

#[test]
fn moves_a_column_before() -> Result {
    let sample = r#"[
        [column1 column2 column3 ... column98  column99  column100];
        [------- ------- ------- --- -------- "   A    " ---------],
        [------- ------- ------- --- -------- "   N    " ---------],
        [------- ------- ------- --- -------- "   D    " ---------],
        [------- ------- ------- --- -------- "   R    " ---------],
        [------- ------- ------- --- -------- "   E    " ---------],
        [------- ------- ------- --- -------- "   S    " ---------]
    ]"#;

    let code = format!(
        r#"
            {sample}
            | move column99 --before column1
            | rename chars
            | get chars
            | str trim
            | str join
        "#
    );

    let actual: String = test().run(code)?;
    assert_contains("ANDRES", actual);
    Ok(())
}

#[test]
fn moves_columns_before() -> Result {
    let sample = r#"[
        [column1 column2  column3  ... column98  column99  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let code = format!(
        r#"
            {sample}
            | move column99 column3 --before column2
            | rename _ chars_1 chars_2
            | select chars_2 chars_1
            | upsert new_col {{|f| $f | transpose | get column1 | str trim | str join}}
            | get new_col
            | str join
        "#
    );

    let actual: String = test().run(code)?;
    assert_contains("ANDRES::JT", actual);
    Ok(())
}

#[test]
fn moves_a_column_after() -> Result {
    let sample = r#"[
        [column1 column2  letters  ... column98  and_more  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let code = format!(
        r#"
            {sample}
            | move letters --after and_more
            | move letters and_more --before column2
            | rename _ chars_1 chars_2
            | select chars_1 chars_2
            | upsert new_col {{|f| $f | transpose | get column1 | str trim | str join}}
            | get new_col
            | str join
        "#
    );

    let actual: String = test().run(code)?;
    assert_contains("ANDRES::JT", actual);
    Ok(())
}

#[test]
fn moves_columns_after() -> Result {
    let content = r#"[
        [column1 column2   letters ... column98  and_more  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let code = format!(
        r#"
            {content}
            | move letters and_more --after column1
            | columns
            | select 1 2
            | str join
        "#
    );

    let actual: String = test().run(code)?;
    assert_contains("lettersand_more", actual);
    Ok(())
}
