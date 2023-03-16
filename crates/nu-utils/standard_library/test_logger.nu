use std.nu assert
use logger.nu *

def run [system_level, message_level] {
    do { nu -c $'use logger.nu; NU_LOG_LEVEL=($system_level) logger log ($message_level) "test message"' } | complete | get -i stderr
}
def "assert no message" [system_level, message_level] {
    let output = (run $system_level $message_level)
    assert ($output == "")
}

def "assert message" [system_level, message_level, message_level_str] {
    let output = (run $system_level $message_level)
    assert ($output | str contains $message_level_str)
    assert ($output | str contains "test message")
}

export def test_critical [] {
    assert no message 99 critical
    assert message CRITICAL critical CRIT
}
export def test_error [] {
    assert no message CRITICAL error 
    assert message ERROR error ERROR
}
export def test_warning [] {
    assert no message ERROR warning 
    assert message  WARNING warning WARN
}
export def test_info [] {
    assert no message WARNING info 
    assert message  INFO info INFO
}
export def test_debug [] {
    assert no message INFO debug 
    assert message  DEBUG debug DEBUG
}
