use nu_test_support::nu;

#[test]
fn changes_the_column_name() {
    let sample = r#"[
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"]
    ]"#;

    let actual = nu!(format!(
        "
            {sample}
            | wrap name
            | rename mosqueteros
            | get mosqueteros
            | length
        "
    ));

    assert_eq!(actual.out, "4");
}

#[test]
fn keeps_remaining_original_names_given_less_new_names_than_total_original_names() {
    let sample = r#"[
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"]
    ]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | wrap name
            | default "arepa!" hit
            | rename mosqueteros
            | get hit
            | length
        "#
    ));

    assert_eq!(actual.out, "4");
}

#[test]
fn errors_if_no_columns_present() {
    let sample = r#"[
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"]
    ]"#;

    let actual = nu!(format!("{sample} | rename mosqueteros"));

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn errors_if_columns_param_is_empty() {
    let sample = r#"[
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"]
    ]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | wrap name
            | default "arepa!" hit
            | rename --column {{}}
        "#
    ));

    assert!(actual.err.contains("The column info cannot be empty"));
}
