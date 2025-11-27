# Assert commands

const indent = '    '

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
    --error-labels: table<text: string, span: record<start: int, end: int>> # Labels for `error make` if you want to create a custom assert
] {
    if not $condition {
        error make {
            msg: ($message | default "Assertion failed."),
            labels: ([...$error_labels $error_label]
                | compact -e
                | default -e [
                {text: "It is not true.", span: (metadata $condition).span}
            ])
        }
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
    --error-labels: table<text: string, span: record<start: int, end: int>> # Labels for `error make` if you want to create a custom assert
] {
    (
        main
        (not $condition)
        ($message | default "Assertion failed.")
        --error-labels ([...$error_labels $error_label] | compact)
    )
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
    let error_raised = (try { do $code } catch { true })
    main ($error_raised == true) $message --error-labels [{
        span: (metadata $code).span
        text: (
            "There were no error during code execution:\n"
         + (view source $code | lines | each {$"($indent)($in)"} | str join (char newline))
        )
    }]
}

# Assert $left == $right
#
# For more documentation see the assert command
@example "This assert passes" { assert equal 1 1 }
@example "This assert passes" { assert equal (0.1 + 0.2) 0.3 }
@example "This assert fails" { assert equal 1 2 }
export def compare [
    comparison: closure
    left: any,
    right: any,
    base_msg: string
    message?: string
] {
    (
        main
        (do $comparison $left $right)
        ([
            $message
            $base_msg
        ] | compact -e | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left == $right
#
# For more documentation see the assert command
@example "This assert passes" { assert equal 1 1 }
@example "This assert passes" { assert equal (0.1 + 0.2) 0.3 }
@example "This assert fails" { assert equal 1 2 }
export def equal [
    left: any,
    right: any,
    message?: string
] {
    (
        main
        ($left == $right)
        ([
            $message
            "These are not equal."
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left != $right
#
# For more documentation see the assert command
@example "This assert passes" { assert not equal 1 2 }
@example "This assert passes" { assert not equal 1 "apple" }
@example "This assert fails" { assert not equal 7 7 }
export def "not equal" [left: any, right: any, message?: string] {
    (
        main
        ($left != $right)
        ([
            $message
            "These are equal"
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left <= $right
#
# For more documentation see the assert command
@example "This assert passes" { assert less or equal 1 2 }
@example "This assert passes" { assert less or equal 1 1 }
@example "This assert fails" { assert less or equal 1 0 }
export def "less or equal" [left: any, right: any, message?: string] {
    (
        main
        ($left <= $right)
        ([
            $message
            "The condition *left <= right* is not satisfied"
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left < $right
#
# For more documentation see the assert command
@example "This assert passes" { assert less 1 2 }
@example "This assert fails" { assert less 1 1 }
export def less [left: any, right: any, message?: string] {
    (
        main
        ($left < $right)
        ([
            $message
            "The condition *left < right* is not satisfied"
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left > $right
#
# For more documentation see the assert command
@example "This assert passes" { assert greater 2 1 }
@example "This assert fails" { assert greater 2 2 }
export def greater [left: any, right: any, message?: string] {
    (
        main
        ($left > $right)
        ([
            $message
            "The condition *left > right* is not satisfied"
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

# Assert $left >= $right
#
# For more documentation see the assert command
@example "This assert passes" { assert greater or equal 2 1 }
@example "This assert passes" { assert greater or equal 2 2 }
@example "This assert fails" { assert greater or equal 1 2 }
export def "greater or equal" [left: any, right: any, message?: string] {
    (
        main
        ($left >= $right)
        ([
            $message
            "The condition *left >= right* is not satisfied"
        ] | compact | str join (char newline))
        --error-labels [
            { text: $"left: ($left | to nuon)" span: (metadata $left).span }
            { text: $"right: ($right | to nuon)" span: (metadata $right).span }
        ]
    )
}

alias "core length" = length
# Assert length of $left is $right
#
# For more documentation see the assert command
@example "This assert passes" { assert length [0, 0] 2 }
@example "This assert fails" { assert length [0] 3 }
export def length [left: list, right: int, message?: string] {
    (
        main
        (($left | core length) == $right)
        ([
            $message
            "`$left` does not have the correct length"
        ] | compact | str join (char newline))
        --error-labels [
            {text: $"expected: ($right)" span: (metadata $right).span}
            {text: $"length: ($left | core length)" span: (metadata $left).span}
        ]
    )
}

alias "core str contains" = str contains
# Assert that ($left | str contains $right)
#
# For more documentation see the assert command
@example "This assert passes" { assert str contains "arst" "rs" }
@example "This assert fails" { assert str contains "arst" "k" }
export def "str contains" [left: string, right: string, message?: string] {
    (
        main
        ($left | core str contains $right)
        ([
            $message
            "`$left` does not have the correct length"
        ] | compact | str join (char newline))
        --error-labels [
            {text: $"expected: ($right)" span: (metadata $right).span}
            {text: $"value: ($left | to nuon)" span: (metadata $left).span}
        ]
    )
}
