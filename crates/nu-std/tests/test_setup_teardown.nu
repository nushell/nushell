use std log
use std assert

def before-each [] {
    log debug "Setup is running"
    {msg: "This is the context"}
}

def after-each [] {
    log debug $"Teardown is running. Context: ($in)"
}

def test_assert_pass [] {
    log debug $"Assert is running. Context: ($in)"
}

def test_assert_skip [] {
    log debug $"Assert is running. Context: ($in)"
    assert skip
}

def test_assert_fail_skipped_by_default [] {
    assert skip # Comment this line if you want to see what happens if a test fails
    log debug $"Assert is running. Context: ($in)"
    assert false
}

def unrelated [] {
    log error "This should not run"
}
