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
