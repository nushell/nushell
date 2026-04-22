use nu_test_support::prelude::*;

#[test]
fn continue_for_loop() -> Result {
    let code = "
        mut vals = []
        for i in 1..10 { if $i == 2 { continue }; $vals ++= [$i] }
        $vals
    ";

    test()
        .run(code)
        .expect_value_eq([1, 3, 4, 5, 6, 7, 8, 9, 10])
}
