use std *

export def test_assert [] {
    assert true
    assert (1 + 2 == 3)
    assert error { assert false }
    assert error { assert (1 + 2 == 4) }
}

export def test_assert_not [] {
    assert not false
    assert not (1 + 2 == 4)
    assert error { assert not true }
    assert error { assert not (1 + 2 == 3) }
}

export def test_assert_equal [] {
    assert equal (1 + 2) 3
    assert equal (0.1 + 0.2 | into string | into decimal) 0.3 # 0.30000000000000004 == 0.3
    assert error { assert equal 1 "foo" }
    assert error { assert equal (1 + 2) 4 }
}

export def test_assert_not_equal [] {
    assert not equal (1 + 2) 4
    assert not equal 1 "foo"
    assert not equal (1 + 2) "3"
    assert error { assert not equal 1 1 }
}

export def test_assert_error [] {
    let failing_code = {|| missing_code_to_run}
    assert error $failing_code

    let good_code = {|| }
    let assert_error_raised = (try { do assert $good_code; false } catch { true })
    assert $assert_error_raised "The assert error should raise an error if there is no error in the executed code."
}

export def test_assert_less [] {
    assert less 1 2
    assert error { assert less 1 1 }
}

export def test_assert_less_or_equal [] {
    assert less or equal 1 2
    assert less or equal 1 1
    assert error { assert less or equal 1 0 }
}

export def test_assert_greater [] {
    assert greater 2 1
    assert error { assert greater 2 2 }
}

export def test_assert_greater_or_equal [] {
    assert greater or equal 1 1
    assert greater or equal 2 1
    assert error { assert greater or equal 0 1 }
}

export def test_assert_length [] {
    assert length [0, 0, 0]  3
    assert error { assert length [0, 0] 3 }
}

export def test_assert_skip [] {
    assert skip # This test case is skipped on purpose
}
