use std log
use std assert

export def setup [] {
    log debug "Setup is running"
    {msg: "This is the context"}
}

export def teardown [] {
    log debug $"Teardown is running. Context: ($in)"
}

export def test_assert_pass [] {
    log debug $"Assert is running. Context: ($in)"
}

export def test_assert_skip [] {
    log debug $"Assert is running. Context: ($in)"
    assert skip
}

export def test_assert_fail_skipped_by_default [] {
    assert skip # Comment this line if you want to see what happens if a test fails
    log debug $"Assert is running. Context: ($in)"
    assert false
}

export def unrelated [] {
    log error "This should not run"
}
