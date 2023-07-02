use std log
use std assert

#[before-each]
def before-each [] {
    log debug "Setup is running"
    {msg: "This is the context"}
}

#[after-each]
def after-each [] {
    log debug $"Teardown is running. Context: ($pipe)"
}

#[test]
def assert_pass [] {
    log debug $"Assert is running. Context: ($pipe)"
}

#[ignore]
def assert_skip [] {
    log debug $"Assert is running. Context: ($pipe)"
}

#[ignore]
def assert_fail_skipped_by_default [] {
    # Change test-skip to test if you want to see what happens if a test fails
    log debug $"Assert is running. Context: ($pipe)"
    assert false
}

def unrelated [] {
    log error "This should not run"
}
