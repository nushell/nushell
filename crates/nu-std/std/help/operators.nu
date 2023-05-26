def get-all-operators [] { return [
    [type, operator, name, description, precedence];

    [Assignment, =, Assign, "Assigns a value to a variable.", 10]
    [Assignment, +=, PlusAssign, "Adds a value to a variable.", 10]
    [Assignment, ++=, AppendAssign, "Appends a list or a value to a variable.", 10]
    [Assignment, -=, MinusAssign, "Subtracts a value from a variable.", 10]
    [Assignment, *=, MultiplyAssign, "Multiplies a variable by a value.", 10]
    [Assignment, /=, DivideAssign, "Divides a variable by a value.", 10]
    [Comparison, ==, Equal, "Checks if two values are equal.", 80]
    [Comparison, !=, NotEqual, "Checks if two values are not equal.", 80]
    [Comparison, <, LessThan, "Checks if a value is less than another.", 80]
    [Comparison, <=, LessThanOrEqual, "Checks if a value is less than or equal to another.", 80]
    [Comparison, >, GreaterThan, "Checks if a value is greater than another.", 80]
    [Comparison, >=, GreaterThanOrEqual, "Checks if a value is greater than or equal to another.", 80]
    [Comparison, =~, RegexMatch, "Checks if a value matches a regular expression.", 80]
    [Comparison, !~, NotRegexMatch, "Checks if a value does not match a regular expression.", 80]
    [Comparison, in, In, "Checks if a value is in a list or string.", 80]
    [Comparison, not-in, NotIn, "Checks if a value is not in a list or string.", 80]
    [Comparison, starts-with, StartsWith, "Checks if a string starts with another.", 80]
    [Comparison, ends-with, EndsWith, "Checks if a string ends with another.", 80]
    [Comparison, not, UnaryNot, "Negates a value or expression.", 0]
    [Math, +, Plus, "Adds two values.", 90]
    [Math, ++, Append, "Appends two lists or a list and a value.", 80]
    [Math, -, Minus, "Subtracts two values.", 90]
    [Math, *, Multiply, "Multiplies two values.", 95]
    [Math, /, Divide, "Divides two values.", 95]
    [Math, //, FloorDivision, "Divides two values and floors the result.", 95]
    [Math, mod, Modulo, "Divides two values and returns the remainder.", 95]
    [Math, **, "Pow ", "Raises one value to the power of another.", 100]
    [Bitwise, bit-or, BitOr, "Performs a bitwise OR on two values.", 60]
    [Bitwise, bit-xor, BitXor, "Performs a bitwise XOR on two values.", 70]
    [Bitwise, bit-and, BitAnd, "Performs a bitwise AND on two values.", 75]
    [Bitwise, bit-shl, ShiftLeft, "Shifts a value left by another.", 85]
    [Bitwise, bit-shr, ShiftRight, "Shifts a value right by another.", 85]
    [Boolean, and, And, "Checks if two values are true.", 50]
    [Boolean, or, Or, "Checks if either value is true.", 40]
    [Boolean, xor, Xor, "Checks if one value is true and the other is false.", 45]
]}

def "nu-complete list-operators" [] {
    let completions = (
        get-all-operators
        | select name description
        | rename value description
    )
    $completions
}

def build-operator-page [operator: record] {
    [
        (build-help-header -n "Description")
        $"    ($operator.description)"
        ""
        (build-help-header -n "Operator")
        $"  ($operator.name) (char lparen)(ansi cyan_bold)($operator.operator)(ansi reset)(char rparen)"
        (build-help-header -n "Type")
        $"  ($operator.type)"
        (build-help-header -n "Precedence")
        $"  ($operator.precedence)"
    ] | str join "\n"
}

# Show help on nushell operators.
#
# Examples:
#     search for string in operators names
#     > help operators --find Bit
#     ╭───┬─────────┬──────────┬────────┬───────────────────────────────────────┬────────────╮
#     │ # │  type   │ operator │  name  │              description              │ precedence │
#     ├───┼─────────┼──────────┼────────┼───────────────────────────────────────┼────────────┤
#     │ 0 │ Bitwise │ bit-and  │ BitAnd │ Performs a bitwise AND on two values. │         75 │
#     │ 1 │ Bitwise │ bit-or   │ BitOr  │ Performs a bitwise OR on two values.  │         60 │
#     │ 2 │ Bitwise │ bit-xor  │ BitXor │ Performs a bitwise XOR on two values. │         70 │
#     ╰───┴─────────┴──────────┴────────┴───────────────────────────────────────┴────────────╯
#
#     search help for single operator
#     > help operators NotRegexMatch
#     Description:
#         Checks if a value does not match a regular expression.
#
#     Operator: NotRegexMatch (!~)
#     Type: Comparison
#     Precedence: 80
#
#     search for an operator that does not exist
#     > help operator "does not exist"
#     Error:
#       × std::help::operator_not_found
#        ╭─[entry #21:1:1]
#      1 │ help operator "does not exist"
#        ·               ────────┬───────
#        ·                       ╰── operator not found
#        ╰────
export def operators [
    ...operator: string@"nu-complete list-operators"  # the name of operator to get help on
    --find (-f): string  # string to find in operator names
] {
    let operators = (get-all-operators)

    if not ($find | is-empty) {
        $operators | find $find --columns [type name]
    } else if not ($operator | is-empty) {
        let found_operator = ($operators | where name == ($operator | str join " "))

        if ($found_operator | is-empty) {
            operator-not-found-error (metadata $operator | get span)
        }

        build-operator-page ($found_operator | get 0)
    } else {
        $operators
    }
}
