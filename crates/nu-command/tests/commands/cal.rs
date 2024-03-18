use nu_test_support::{nu, pipeline};

#[test]
fn cal_full_year() {
    let actual = nu!("cal -y --full-year 2010 | first | to json -r");

    let first_week_2010_json =
        r#"{"year":2010,"su":null,"mo":null,"tu":null,"we":null,"th":null,"fr":1,"sa":2}"#;

    assert_eq!(actual.out, first_week_2010_json);
}

#[test]
fn cal_february_2020_leap_year() {
    let actual = nu!(pipeline(
        r#"
        cal -ym --full-year 2020 --month-names | where month == "february" | to json -r
        "#
    ));

    let cal_february_json = r#"[{"year":2020,"month":"february","su":null,"mo":null,"tu":null,"we":null,"th":null,"fr":null,"sa":1},{"year":2020,"month":"february","su":2,"mo":3,"tu":4,"we":5,"th":6,"fr":7,"sa":8},{"year":2020,"month":"february","su":9,"mo":10,"tu":11,"we":12,"th":13,"fr":14,"sa":15},{"year":2020,"month":"february","su":16,"mo":17,"tu":18,"we":19,"th":20,"fr":21,"sa":22},{"year":2020,"month":"february","su":23,"mo":24,"tu":25,"we":26,"th":27,"fr":28,"sa":29}]"#;

    assert_eq!(actual.out, cal_february_json);
}

#[test]
fn cal_fr_the_thirteenths_in_2015() {
    let actual = nu!(pipeline(
        r#"
        cal --full-year 2015 | default 0 fr | where fr == 13 | length
        "#
    ));

    assert!(actual.out.contains('3'));
}

#[test]
fn cal_rows_in_2020() {
    let actual = nu!(pipeline(
        r#"
        cal --full-year 2020 | length
        "#
    ));

    assert!(actual.out.contains("62"));
}

#[test]
fn cal_week_day_start_mo() {
    let actual = nu!(pipeline(
        r#"
        cal --full-year 2020 -m --month-names --week-start mo | where month == january | to json -r
        "#
    ));

    let cal_january_json = r#"[{"month":"january","mo":null,"tu":null,"we":1,"th":2,"fr":3,"sa":4,"su":5},{"month":"january","mo":6,"tu":7,"we":8,"th":9,"fr":10,"sa":11,"su":12},{"month":"january","mo":13,"tu":14,"we":15,"th":16,"fr":17,"sa":18,"su":19},{"month":"january","mo":20,"tu":21,"we":22,"th":23,"fr":24,"sa":25,"su":26},{"month":"january","mo":27,"tu":28,"we":29,"th":30,"fr":31,"sa":null,"su":null}]"#;

    assert_eq!(actual.out, cal_january_json);
}

#[test]
fn cal_sees_pipeline_year() {
    let actual = nu!(pipeline(
        r#"
        cal --full-year 1020 | get mo | first 4 | to json -r
        "#
    ));

    assert_eq!(actual.out, "[null,3,10,17]");
}
