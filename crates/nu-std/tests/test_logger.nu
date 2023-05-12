use std *

def run [
    system_level,
    message_level
    --short (-s)
] {
    do {
        if $short {
            ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log ($message_level) --short "test message"'
        } else {
            ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log ($message_level) "test message"'
        }
    } | complete | get --ignore-errors stderr
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

export def test_critical [] {
    assert no message 99 critical
    assert message CRITICAL critical CRT
}

export def test_critical_short [] {
    assert message short CRITICAL critical C
}

export def test_error [] {
    assert no message CRITICAL error 
    assert message ERROR error ERR
}

export def test_error_short [] {
    assert message short ERROR error E
}

export def test_warning [] {
    assert no message ERROR warning 
    assert message WARNING warning WRN
}

export def test_warning_short [] {
    assert message short WARNING warning W
}

export def test_info [] {
    assert no message WARNING info 
    assert message INFO info "INF" # INF has to be quoted, otherwise it is the `inf` float
}

export def test_info_short [] {
    assert message short INFO info I
}

export def test_debug [] {
    assert no message INFO debug 
    assert message DEBUG debug DBG
}

export def test_debug_short [] {
    assert message short DEBUG debug D
}


def "run custom" [
    system_level,
    format,
    message_level
] {
    do {
        ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "test message" "($format)" ($message_level)' 
    } | complete | get --ignore-errors stderr
}

def "assert custom message" [
    system_level,
    format,
    message_level
] {
    let output = (run custom $system_level $format $message_level)
    assert equal ($output | str trim --right) ($format | str replace "%MSG%" "test message")
}

def "assert custom message contains" [
    system_level,
    format,
    message_level,
    tested_str
] {
    let output = (run custom $system_level $format $message_level)
    assert ($output | str contains  $tested_str)
    assert ($output | str contains "test message")
}

def "assert custom message not contains" [
    system_level,
    format,
    message_level,
    tested_str
] {
    let output = (run custom $system_level $format $message_level)
    assert (not ($output | str contains  $tested_str))
    assert ($output | str contains "test message")
}

def "assert no custom message" [
    system_level,
    format,
    message_level
] {
    let output = (run custom $system_level $format $message_level)
    assert equal ($output | str trim --right) ""
}

export def test_custom [] {
    assert no custom message (log ERROR_LEVEL) "%MSG%" (log DEBUG_LEVEL)
    assert custom message (log DEBUG_LEVEL) "%MSG%" (log INFO_LEVEL)
    assert custom message (log WARNING_LEVEL) $"my_msg: %MSG%" (log CRITICAL_LEVEL)

    assert custom message contains (log DEBUG_LEVEL) $"(ansi yellow)[%LEVEL%]MY MESSAGE: %MSG% [%DATE%](ansi reset)" (log WARNING_LEVEL) (log WARNING_LEVEL_PREFIX)
    assert custom message not contains (log DEBUG_LEVEL) $"(ansi yellow)MY MESSAGE: %MSG% [%DATE%](ansi reset)" (log WARNING_LEVEL) (log WARNING_LEVEL_PREFIX)
}

export def "test_long_prefixes" [] {
    assert equal (log CRITICAL_LEVEL_PREFIX) "CRT"
    assert equal (log ERROR_LEVEL_PREFIX) "ERR"
    assert equal (log WARNING_LEVEL_PREFIX) "WRN"
    assert equal (log INFO_LEVEL_PREFIX) "INF"
    assert equal (log DEBUG_LEVEL_PREFIX) "DBG"
}

export def "test_short_prefixes" [] {
    assert equal (log CRITICAL_LEVEL_PREFIX --short) "C"
    assert equal (log ERROR_LEVEL_PREFIX --short) "E"
    assert equal (log WARNING_LEVEL_PREFIX --short) "W"
    assert equal (log INFO_LEVEL_PREFIX --short) "I"
    assert equal (log DEBUG_LEVEL_PREFIX --short) "D"
}
