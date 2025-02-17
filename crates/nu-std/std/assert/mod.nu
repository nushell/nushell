##################################################################################
#
# Assert commands.
#
##################################################################################

# Universal assert command
#
# If the condition is not true, it generates an error.
@example "This assert passes" { assert (3 == 3) }
@example "This assert fails" { assert (42 == 3) }
@example "The --error-label flag can be used if you want to create a custom assert command:" {
    def "assert even" [number: int] {
        assert ($number mod 2 == 0) --error-label {
            text: $"($number) is not an even number",
            span: (metadata $number).span,
        }
    }
}
export def main [
    condition: bool, # Condition, which should be true
    message?: string, # Optional error message
    --error-label: record<text: string, span: record<start: int, end: int>> # Label for `error make` if you want to create a custom assert
] {
    if $condition { return }
    error make {
        msg: ($message | default "Assertion failed."),
        label: ($error_label | default {
            text: "It is not true.",
            span: (metadata $condition).span,
        })
    }
}

# Negative assertion
#
# If the condition is not false, it generates an error.
@example "This assert passes" { assert (42 == 3) }
@example "This assert fails" { assert (3 == 3) }
@example "The --error-label flag can be used if you want to create a custom assert command:" {
    def "assert not even" [number: int] {
        assert not ($number mod 2 == 0) --error-label {
            span: (metadata $number).span,
            text: $"($number) is an even number",
        }
    }
}
export def not [
    condition: bool, # Condition, which should be false
    message?: string, # Optional error message
    --error-label: record<text: string, span: record<start: int, end: int>> # Label for `error make` if you want to create a custom assert
] {
    if $condition {
        let span = (metadata $condition).span
        error make {
            msg: ($message | default "Assertion failed."),
            label: ($error_label | default {
                text: "It is not false.",
                span: $span,
            })
        }
    }
}


# Assert that executing the code generates an error
#
# For more documentation see the assert command
@example "This assert passes" { assert error {|| missing_command} }
@example "This assert fails" { assert error {|| 12} }
export def error [
    code: closure,
    message?: string
] {
    let error_raised = (try { do $code; false } catch { true })
    main ($error_raised) $message --error-label {
        span: (metadata $code).span
        text: (
            "There were no error during code execution:\n"
         + $"        (view source $code)"
        )
    }
}

# Assert $left == $right
#
# For more documentation see the assert command
@example "This assert passes" { assert equal 1 1 }
@example "This assert passes" { assert equal (0.1 + 0.2) 0.3 }
@example "This assert fails" { assert equal 1 2 }
export def equal [left: any, right: any, message?: string] {
    main ($left == $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "These are not equal.\n"
         + $"        Left  : '($left | to nuon --raw)'\n"
         + $"        Right : '($right | to nuon --raw)'"
        )
    }
}

# Assert $left != $right
#
# For more documentation see the assert command
@example "This assert passes" { assert not equal 1 2 }
@example "This assert passes" { assert not equal 1 "apple" }
@example "This assert fails" { assert not equal 7 7 }
export def "not equal" [left: any, right: any, message?: string] {
    main ($left != $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: $"These are both '($left | to nuon --raw)'."
    }
}

# Assert $left <= $right
#
# For more documentation see the assert command
@example "This assert passes" { assert less or equal 1 2 }
@example "This assert passes" { assert less or equal 1 1 }
@example "This assert fails" { assert less or equal 1 0 }
export def "less or equal" [left: any, right: any, message?: string] {
    main ($left <= $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "The condition *left <= right* is not satisfied.\n"
         + $"        Left  : '($left)'\n"
         + $"        Right : '($right)'"
        )
    }
}

# Assert $left < $right
#
# For more documentation see the assert command
@example "This assert passes" { assert less 1 2 }
@example "This assert fails" { assert less 1 1 }
export def less [left: any, right: any, message?: string] {
    main ($left < $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "The condition *left < right* is not satisfied.\n"
         + $"        Left  : '($left)'\n"
         + $"        Right : '($right)'"
        )
    }
}

# Assert $left > $right
#
# For more documentation see the assert command
@example "This assert passes" { assert greater 2 1 }
@example "This assert fails" { assert greater 2 2 }
export def greater [left: any, right: any, message?: string] {
    main ($left > $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "The condition *left > right* is not satisfied.\n"
         + $"        Left  : '($left)'\n"
         + $"        Right : '($right)'"
        )
    }
}

# Assert $left >= $right
#
# For more documentation see the assert command
@example "This assert passes" { assert greater or equal 2 1 }
@example "This assert passes" { assert greater or equal 2 2 }
@example "This assert fails" { assert greater or equal 1 2 }
export def "greater or equal" [left: any, right: any, message?: string] {
    main ($left >= $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "The condition *left < right* is not satisfied.\n"
         + $"        Left  : '($left)'\n"
         + $"        Right : '($right)'"
        )
    }
}

alias "core length" = length
# Assert length of $left is $right
#
# For more documentation see the assert command
@example "This assert passes" { assert length [0, 0] 2 }
@example "This assert fails" { assert length [0] 3 }
export def length [left: list, right: int, message?: string] {
    main (($left | core length) == $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            "This does not have the correct length:\n"
         + $"        value    : ($left | to nuon --raw)\n"
         + $"        length   : ($left | core length)\n"
         + $"        expected : ($right)"
        )
    }
}

alias "core str contains" = str contains
# Assert that ($left | str contains $right)
#
# For more documentation see the assert command
@example "This assert passes" { assert str contains "arst" "rs" }
@example "This assert fails" { assert str contains "arst" "k" }
export def "str contains" [left: string, right: string, message?: string] {
    main ($left | core str contains $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: (
            $"This does not contain '($right)'.\n"
          + $"        value: ($left | to nuon --raw)"
        )
    }
}
