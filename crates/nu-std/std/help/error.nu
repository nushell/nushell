export def error-fmt [] {
    $"(ansi red)($in)(ansi reset)"
}

def throw-error [error: string, msg: string, span: record] {
    error make {
        msg: ($error | error-fmt)
        label: {
            text: $msg
            start: $span.start
            end: $span.end
        }
    }
}

export def module-not-found-error [span: record] {
    throw-error "std::help::module_not_found" "module not found" $span
}

export def alias-not-found-error [span: record] {
    throw-error "std::help::alias_not_found" "alias not found" $span
}

export def extern-not-found-error [span: record] {
    throw-error "std::help::extern_not_found" "extern not found" $span
}

export def operator-not-found-error [span: record] {
    throw-error "std::help::operator_not_found" "operator not found" $span
}

export def command-not-found-error [span: record] {
    throw-error "std::help::command_not_found" "command not found" $span
}
