##################################################################################
#
# Assert commands.
#
##################################################################################

const main_examples = [
    {
        description: "Pass",
        example: "assert (3 == 3)",
    }
    {
        description: "Fail",
        example: "assert (42 == 3)",
    }
    {
        description: "The --error-label flag can be used if you want to create a custom assert command:",
        example: r#'def "assert even" [number: int] {
        assert ($number mod 2 == 0) --error-label {
            text: $"($number) is not an even number",
            span: (metadata $number).span,
        }
    }'#,
    }
]

# Universal assert command
#
# If the condition is not true, it generates an error.
export def --examples=$main_examples main [
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

const not_examples = [
    {
        description: "Pass",
        example: "assert (42 == 3)",
    }
    {
        description: "Fail",
        example: "assert (3 == 3)",
    }
    {
        description: "The --error-label flag can be used if you want to create a custom assert command:",
        example: r#'def "assert not even" [number: int] {
        assert not ($number mod 2 == 0) --error-label {
            span: (metadata $number).span,
            text: $"($number) is an even number",
        }
    }'#,
    }
]

# Negative assertion
#
# If the condition is not false, it generates an error.
export def --examples=$not_examples not [
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

const error_examples = [
    {
        description: "Pass",
        example: "assert error {|| missing_command}",
    }
    {
        description: "Fail",
        example: "assert error {|| 12}",
    }
]

# Assert that executing the code generates an error
#
# For more documentation see the assert command
export def --examples=$error_examples error [
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

const equal_examples = [
    {
        description: "Pass"
        example: "assert equal 1 1"
    }
    {
        description: ""
        example: "assert equal (0.1 + 0.2) 0.3"
    }
    {
        description: "Fail"
        example: "assert equal 1 2"
    }
]

# Assert $left == $right
#
# For more documentation see the assert command
export def --examples=$equal_examples equal [left: any, right: any, message?: string] {
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

const not_equal_examples = [
    {
        description: "Pass",
        example: r#'assert not equal 1 2'#,
    }
    {
        description: "Pass",
        example: r#'assert not equal 1 "apple"'#,
    }
    {
        description: "Fail",
        example: r#'assert not equal 7 7'#,
    }
]

# Assert $left != $right
#
# For more documentation see the assert command
export def --examples=$not_equal_examples "not equal" [left: any, right: any, message?: string] {
    main ($left != $right) $message --error-label {
        span: {
            start: (metadata $left).span.start
            end: (metadata $right).span.end
        }
        text: $"These are both '($left | to nuon --raw)'."
    }
}

const leq_examples = [
    {
        description: "Pass",
        example: r#'assert less or equal 1 2 # passes'#,
    }
    {
        description: "Pass",
        example: r#'assert less or equal 1 1 # passes'#,
    }
    {
        description: "Fail",
        example: r#'assert less or equal 1 0 # fails'#,
    }
]

# Assert $left <= $right
#
# For more documentation see the assert command
export def --examples=$leq_examples "less or equal" [left: any, right: any, message?: string] {
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

const less_examples = [
    {
        description: "Pass"
        example: r#'assert less 1 2'#,
    }
    {
        description: "Fail"
        example: r#'assert less 1 1'#
    }
]

# Assert $left < $right
#
# For more documentation see the assert command
export def --examples=$less_examples less [left: any, right: any, message?: string] {
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

const greater_examples = [
    {
        description: "Pass",
        example: r#'assert greater 2 1'#,
    }
    {
        description: "Fail",
        example: r#'assert greater 2 2'#,
    }
]

# Assert $left > $right
#
# For more documentation see the assert command
export def --examples=$greater_examples greater [left: any, right: any, message?: string] {
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


const geq_examples = [
    {
        description: "Pass",
        example: r#'assert greater or equal 2 1'#,
    }
    {
        description: "Pass",
        example: r#'assert greater or equal 2 2'#,
    }
    {
        description: "Fail",
        example: r#'assert greater or equal 1 2'#,
    }
]

# Assert $left >= $right
#
# For more documentation see the assert command
export def --examples=$geq_examples "greater or equal" [left: any, right: any, message?: string] {
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

const length_examples = [
    {
        description: "Pass",
        example: r#'assert length [0, 0] 2'#,
    }
    {
        description: "Fail",
        example: r#'assert length [0] 3'#,
    }
]

alias "core length" = length
# Assert length of $left is $right
#
# For more documentation see the assert command
export def --examples=$length_examples length [left: list, right: int, message?: string] {
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

const str_constains_examples = [
    {
        description: "Pass"
        example: r#'assert str contains "arst" "rs"'#
    }
    {
        description: "Fail"
        example: r#'assert str contains "arst" "k"'#
    }
]

alias "core str contains" = str contains
# Assert that ($left | str contains $right)
#
# For more documentation see the assert command
export def --examples=$str_constains_examples "str contains" [left: string, right: string, message?: string] {
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
