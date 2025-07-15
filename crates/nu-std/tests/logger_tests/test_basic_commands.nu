use std/testing *
use std/assert

def run [
    system_level,
    message_level
    --short
] {
    if $short {
        ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log ($message_level) --short "test message"'
    } else {
        ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log ($message_level) "test message"'
    }
    | complete | get --optional stderr
}

def "assert no message" [
    system_level,
    message_level
] {
    let output = (run $system_level $message_level)
    assert equal "" $output
}

def "assert message" [
    system_level,
    message_level,
    message_level_str
] {
    let output = (run $system_level $message_level)
    assert str contains $output $message_level_str
    assert str contains $output "test message"
}

def "assert message short" [
    system_level,
    message_level,
    message_level_str
] {
    let output = (run --short $system_level $message_level)
    assert str contains $output $message_level_str
    assert str contains $output "test message"
}

@test
def critical [] {
    assert no message 99 critical
    assert message CRITICAL critical CRT
}

@test
def critical_short [] {
    assert message short CRITICAL critical C
}

@test
def error [] {
    assert no message CRITICAL error
    assert message ERROR error ERR
}

@test
def error_short [] {
    assert message short ERROR error E
}

@test
def warning [] {
    assert no message ERROR warning
    assert message WARNING warning WRN
}

@test
def warning_short [] {
    assert message short WARNING warning W
}

@test
def info [] {
    assert no message WARNING info
    assert message INFO info "INF" # INF has to be quoted, otherwise it is the `inf` float
}

@test
def info_short [] {
    assert message short INFO info I
}

@test
def debug [] {
    assert no message INFO debug
    assert message DEBUG debug DBG
}

@test
def debug_short [] {
    assert message short DEBUG debug D
}
