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
    assert str contains $output $tested_str
    assert str contains $output "test message"
}

def "assert custom message not contains" [
    system_level,
    format,
    message_level,
    tested_str
] {
    let output = (run custom $system_level $format $message_level)
    assert not ($output | str contains  $tested_str)
    assert str contains $output "test message"
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
    assert no custom message $env.LOG_LEVEL.ERROR "%MSG%" $env.LOG_LEVEL.DEBUG
    assert custom message $env.LOG_LEVEL.DEBUG "%MSG%" $env.LOG_LEVEL.INFO
    assert custom message $env.LOG_LEVEL.WARNING $"my_msg: %MSG%" $env.LOG_LEVEL.CRITICAL

    assert custom message contains $env.LOG_LEVEL.DEBUG $"(ansi yellow)[%LEVEL%]MY MESSAGE: %MSG% [%DATE%](ansi reset)" $env.LOG_LEVEL.WARNING $env.LOG_PREFIX.WARNING
    assert custom message not contains $env.LOG_LEVEL.DEBUG $"(ansi yellow)MY MESSAGE: %MSG% [%DATE%](ansi reset)" $env.LOG_LEVEL.WARNING $env.LOG_PREFIX.WARNING
}

export def "test_env_log_ansi" [] {
    assert equal $env.LOG_ANSI.CRITICAL (ansi red_bold)
    assert equal $env.LOG_ANSI.ERROR (ansi red)
    assert equal $env.LOG_ANSI.WARNING (ansi yellow)
    assert equal $env.LOG_ANSI.INFO (ansi default)
    assert equal $env.LOG_ANSI.DEBUG (ansi default_dimmed)
}

export def "test_env_log_level" [] {
    assert equal $env.LOG_LEVEL.CRITICAL 50
    assert equal $env.LOG_LEVEL.ERROR 40
    assert equal $env.LOG_LEVEL.WARNING 30
    assert equal $env.LOG_LEVEL.INFO 20
    assert equal $env.LOG_LEVEL.DEBUG 10
}

export def "test_env_log_prefix" [] {
    assert equal $env.LOG_PREFIX.CRITICAL "CRT"
    assert equal $env.LOG_PREFIX.ERROR "ERR"
    assert equal $env.LOG_PREFIX.WARNING "WRN"
    assert equal $env.LOG_PREFIX.INFO "INF"
    assert equal $env.LOG_PREFIX.DEBUG "DBG"
}

export def "test_env_log_short_prefix" [] {
    assert equal $env.LOG_SHORT_PREFIX.CRITICAL "C"
    assert equal $env.LOG_SHORT_PREFIX.ERROR "E"
    assert equal $env.LOG_SHORT_PREFIX.WARNING "W"
    assert equal $env.LOG_SHORT_PREFIX.INFO "I"
    assert equal $env.LOG_SHORT_PREFIX.DEBUG "D"
}

export def "test_env_log_format" [] {
    assert equal $env.LOG_FORMAT $"%ANSI_START%%DATE%|%LEVEL%|(ansi u)%MSG%%ANSI_STOP%"
}