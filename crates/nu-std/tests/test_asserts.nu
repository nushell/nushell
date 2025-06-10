use std/testing *
use std *
use std/assert

@test
def assert_basic [] {
    assert true
    assert (1 + 2 == 3)
    assert error { assert false }
    assert error { assert (1 + 2 == 4) }
}

@test
def assert_not [] {
    assert not false
    assert not (1 + 2 == 4)
    assert error { assert not true }
    assert error { assert not (1 + 2 == 3) }
}

@test
def assert_equal [] {
    assert equal (1 + 2) 3
    assert equal (0.1 + 0.2 | into string | into float) 0.3 # 0.30000000000000004 == 0.3
    assert error { assert equal 1 "foo" }
    assert error { assert equal (1 + 2) 4 }
}

@test
def assert_not_equal [] {
    assert not equal (1 + 2) 4
    assert not equal 1 "foo"
    assert not equal (1 + 2) "3"
    assert error { assert not equal 1 1 }
}

@test
def assert_error [] {
    let failing_code = {|| missing_code_to_run}
    assert error $failing_code

    let good_code = {|| }
    let assert_error_raised = (try { assert error $good_code; false } catch { true })
    assert $assert_error_raised "The assert error should be false if there is no error in the executed code."
}

@test
def assert_less [] {
    assert less 1 2
    assert error { assert less 1 1 }
}

@test
def assert_less_or_equal [] {
    assert less or equal 1 2
    assert less or equal 1 1
    assert error { assert less or equal 1 0 }
}

@test
def assert_greater [] {
    assert greater 2 1
    assert error { assert greater 2 2 }
}

@test
def assert_greater_or_equal [] {
    assert greater or equal 1 1
    assert greater or equal 2 1
    assert error { assert greater or equal 0 1 }
}

@test
def assert_length [] {
    assert length [0, 0, 0]  3
    assert error { assert length [0, 0] 3 }
}

@ignore
def assert_skip [] {
    assert true # This test case is skipped on purpose
}
