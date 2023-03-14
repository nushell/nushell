use std.nu assert
use logger.nu *

def "assert no message" [output: string] {
    assert (($output | describe) == "nothing")
}

def "assert message" [level_str: string, output:string] {
    assert ($output | str contains $level_str)
    assert ($output | str contains "test message")
}

export def test_critical [] {
    assert no message (NU_LOG_LEVEL=99 log critical  "test message")
    assert message CRIT (NU_LOG_LEVEL=CRITICAL log critical "test message")
}
export def test_error [] {
    assert no message (NU_LOG_LEVEL=CRITICAL log error "test message")
    assert message ERROR (NU_LOG_LEVEL=ERROR log error "test message")
}
export def test_warn [] {
    assert no message (NU_LOG_LEVEL=ERROR log warning "test message")
    assert message WARN (NU_LOG_LEVEL=WARNING log warning "test message")
}
export def test_info [] {
    assert no message (NU_LOG_LEVEL=WARNING log info "test message")
    assert message INFO (NU_LOG_LEVEL=INFO log info "test message")
}
export def test_debug [] {
    assert no message (NU_LOG_LEVEL=INFO log debug "test message")
    assert message DEBUG (NU_LOG_LEVEL=DEBUG log debug "test message")
}

export def example_log [] {
    log critical "this is a critical message"
    log error "this is an error message"
    log warning "this is a warning message"
    log info "this is an info message"
    log debug "this is a debug message"
}
