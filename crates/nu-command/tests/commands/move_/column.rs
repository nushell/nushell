use nu_test_support::nu;

#[test]
fn moves_a_column_before() {
    let sample = r#"[
        [column1 column2 column3 ... column98  column99  column100];
        [------- ------- ------- --- -------- "   A    " ---------],
        [------- ------- ------- --- -------- "   N    " ---------],
        [------- ------- ------- --- -------- "   D    " ---------],
        [------- ------- ------- --- -------- "   R    " ---------],
        [------- ------- ------- --- -------- "   E    " ---------],
        [------- ------- ------- --- -------- "   S    " ---------]
    ]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | move column99 --before column1
            | rename chars
            | get chars
            | str trim
            | str join
        "#
    ));

    assert!(actual.out.contains("ANDRES"));
}

#[test]
fn moves_columns_before() {
    let sample = r#"[
        [column1 column2  column3  ... column98  column99  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | move column99 column3 --before column2
            | rename _ chars_1 chars_2
            | select chars_2 chars_1
            | upsert new_col {{|f| $f | transpose | get column1 | str trim | str join}}
            | get new_col
            | str join
        "#
    ));

    assert!(actual.out.contains("ANDRES::JT"));
}

#[test]
fn moves_a_column_after() {
    let sample = r#"[
        [column1 column2  letters  ... column98  and_more  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let actual = nu!(format!(
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
    ));

    assert!(actual.out.contains("ANDRES::JT"));
}

#[test]
fn moves_columns_after() {
    let content = r#"[
        [column1 column2   letters ... column98  and_more  column100];
        [------- ------- "   A   " --- -------- "   N    " ---------]
        [------- ------- "   D   " --- -------- "   R    " ---------]
        [------- ------- "   E   " --- -------- "   S    " ---------]
        [------- ------- "   :   " --- -------- "   :    " ---------]
        [------- ------- "   J   " --- -------- "   T    " ---------]
    ]"#;

    let actual = nu!(format!(
        r#"
            {content}
            | move letters and_more --after column1
            | columns
            | select 1 2
            | str join
        "#
    ));

    assert!(actual.out.contains("lettersand_more"));
}
