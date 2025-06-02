use nu_test_support::{nu, pipeline};

#[test]
fn filter_with_return_in_closure() {
    let actual = nu!(pipeline(
        "
        1..10 | filter { |it|
            if $it mod 2 == 0 {
                return true
            };
            return false;
        } | to nuon
        "
    ));

    assert_eq!(actual.out, "[2, 4, 6, 8, 10]");
    assert!(actual.err.contains("deprecated"));
}
