use std *

def run [system_level, message_level] {
    do {
        ^$nu.current-exe -c $'use std; NU_LOG_LEVEL=($system_level) std log ($message_level) "test message"' 
    } | complete | get -i stderr
}
def "assert no message" [system_level, message_level] {
    let output = (run $system_level $message_level)
    assert equal "" $output
}

def "assert message" [system_level, message_level, message_level_str] {
    let output = (run $system_level $message_level)
    assert str contains $output $message_level_str
    assert str contains $output "test message"
}

export def test_critical [] {
    assert no message 99 critical
    assert message CRITICAL critical CRT
}
export def test_error [] {
    assert no message CRITICAL error 
    assert message ERROR error ERR
}
export def test_warning [] {
    assert no message ERROR warning 
    assert message WARNING warning WRN
}
export def test_info [] {
    assert no message WARNING info 
    assert message INFO info "INF" #INF has to be quoted, otherwise it is the `inf` float
}
export def test_debug [] {
    assert no message INFO debug 
    assert message DEBUG debug DBG
}


def "run custom" [system_level, format, message_level] {
    do {
        ^$nu.current-exe -c $'use std; NU_LOG_LEVEL=($system_level) std log custom "test message" "($format)" ($message_level)' 
    } | complete | get -i stderr
}

def "assert custom message" [system_level, format, message_level] {
    let output = (run custom $system_level $format $message_level)
    assert equal ($output | str trim -r) ($format | str replace "%MSG%" "test message")
}

def "assert custom message contains" [system_level, format, message_level, tested_str] {
    let output = (run custom $system_level $format $message_level)
    assert ($output | str contains  $tested_str)
    assert ($output | str contains "test message")
}

def "assert custom message not contains" [system_level, format, message_level, tested_str] {
    let output = (run custom $system_level $format $message_level)
    assert (not ($output | str contains  $tested_str))
    assert ($output | str contains "test message")
}

def "assert no custom message" [system_level, format, message_level] {
    let output = (run custom $system_level $format $message_level)
    assert equal ($output | str trim -r) ""
}

export def test_custom [] {
    assert no custom message (log ERROR_LEVEL) "%MSG%" (log DEBUG_LEVEL)
    assert custom message (log DEBUG_LEVEL) "%MSG%" (log INFO_LEVEL)
    assert custom message (log WARNING_LEVEL) $"my_msg: %MSG%" (log CRITICAL_LEVEL)

    assert custom message contains (log DEBUG_LEVEL) $"(ansi yellow)[%LEVEL%]MY MESSAGE: %MSG% [%DATE%](ansi reset)" (log WARNING_LEVEL) (log WARNING_LEVEL_PREFIX)
    assert custom message not contains (log DEBUG_LEVEL) $"(ansi yellow)MY MESSAGE: %MSG% [%DATE%](ansi reset)" (log WARNING_LEVEL) (log WARNING_LEVEL_PREFIX)
}