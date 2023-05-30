use std *

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