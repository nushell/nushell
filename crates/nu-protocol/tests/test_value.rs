use nu_protocol::{NuDuration, Span, Unit, Value};
use rstest::rstest;
use std::cmp::Ordering;

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::test_int(1),
        Value::test_string("string"),
        Value::test_float(1.0),
    ];

    let nothing = Value::Nothing {
        span: Span::test_data(),
    };

    for value in values {
        assert!(matches!(
            value.eq(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            value.ne(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));

        assert!(matches!(
            nothing.eq(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            nothing.ne(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));
    }
}

#[rstest]
#[case(
    Value::test_float(100.0),
    Value::test_duration(NuDuration::new(100, Unit::Nanosecond)),
    Some(Ordering::Equal)
)]
#[case(
    Value::test_int(100),
    Value::test_duration(NuDuration::new(100, Unit::Nanosecond)),
    Some(Ordering::Equal)
)]
#[case(
    Value::test_duration(NuDuration::new(100, Unit::Nanosecond)),
    Value::test_duration(NuDuration::new(100, Unit::Nanosecond)),
    Some(Ordering::Equal)
)]
#[case(
    Value::test_int(100),
    Value::test_duration(NuDuration::new(100, Unit::Year)),
    None
)]
#[case(
    Value::test_int(100),
    Value::test_duration(NuDuration::new(i64::MAX, Unit::Week)),
    None
)]

fn test_partial_cmp_duration(
    #[case] lhs: Value,
    #[case] rhs: Value,
    #[case] exp_result: Option<Ordering>,
) {
    assert_eq!(
        exp_result,
        lhs.partial_cmp(&rhs),
        "lhs::rhs: expected matches observed"
    );

    let reversed_exp_result = match exp_result {
        Some(Ordering::Greater) => Some(Ordering::Less),
        Some(Ordering::Equal) => Some(Ordering::Equal),
        Some(Ordering::Less) => Some(Ordering::Greater),
        None => None,
    };
    assert_eq!(
        reversed_exp_result,
        rhs.partial_cmp(&lhs),
        "rhs::lhs: reversed expected matches observed"
    );
}

#[rstest]
#[case(NuDuration::new(20, Unit::Nanosecond), 45.3, 65.3)]
#[case(NuDuration::new(20, Unit::Second), 45.3, 20_000_000_045.3)]
// verify all 4 permutations of duration +/- float produce expected result
fn test_duration_plus_minus_num(
    #[case] dur: NuDuration,
    #[case] val: f64,
    #[case] exp_result: f64,
) {
    assert_eq!(
        Value::test_duration(dur)
            .add(
                Span::test_data(),
                &Value::test_float(val),
                Span::test_data()
            )
            .expect("foo"),
        Value::test_float(exp_result),
        "dur + float -> exp float"
    );
    assert_eq!(
        Value::test_float(val)
            .add(
                Span::test_data(),
                &Value::test_duration(dur),
                Span::test_data()
            )
            .expect("bar"),
        Value::test_float(exp_result),
        "float + dur -> exp float"
    );

    assert_eq!(
        Value::test_duration(-dur)
            .sub(
                Span::test_data(),
                &Value::test_float(-val),
                Span::test_data()
            )
            .expect("bas"),
        Value::test_float(-exp_result),
        "-dur - -float -> - exp_float"
    );
    assert_eq!(
        Value::test_float(-val)
            .sub(
                Span::test_data(),
                &Value::test_duration(-dur),
                Span::test_data()
            )
            .expect("barf"),
        Value::test_float(-exp_result),
        "-float - -dur -> -exp float"
    );
}
