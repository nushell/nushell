use nu_test_support::nu;

#[test]
fn append_assign_int() {
    let actual = nu!(r#"
            mut a = [1 2];
            $a ++= [3 4];
            $a == [1 2 3 4]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_string() {
    let actual = nu!(r#"
            mut a = [a b];
            $a ++= [c d];
            $a == [a b c d]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_any() {
    let actual = nu!(r#"
            mut a = [1 2 a];
            $a ++= [b 3];
            $a == [1 2 a b 3]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_both_empty() {
    let actual = nu!(r#"
            mut a = [];
            $a ++= [];
            $a == []
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_type_mismatch() {
    let actual = nu!(r#"
            mut a = [1 2];
            $a ++= [a];
            $a == [1 2 "a"]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_single_element() {
    let actual = nu!(r#"
            mut a = ["list" "and"];
            $a ++= "a single element";
	    $a == ["list" "and" "a single element"]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_to_single_element() {
    let actual = nu!(r#"
            mut a = "string";
            $a ++= ["and" "the" "list"];
	    $a == ["string" "and" "the" "list"]
        "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn append_assign_single_to_single() {
    let actual = nu!(r#"
            mut a = 1;
            $a ++= "and a single element";
        "#);

    assert!(actual.err.contains("nu::parser::unsupported_operation"));
}
