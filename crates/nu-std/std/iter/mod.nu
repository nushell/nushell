# | Filter Extensions
# 
# This module implements extensions to the `filters` commands
#
# They are prefixed with `iter` so as to avoid conflicts with 
# the inbuilt filters

const find_examples = [
    {
        description: "Find an element starting with 'a'",
        example: r#'["shell", "abc", "around", "nushell", "std"] | iter find {|e| $e starts-with "a" }'#,
        result: "abc",
    }
    {
        description: "Find an element starting with 'a'",
        example: r#'["shell", "abc", "around", "nushell", "std"] | iter find {|e| $e mod 2 == 0}'#,
        result: null,
    }
]

# Returns the first element of the list that matches the
# closure predicate, `null` otherwise
#
# # Invariant
# > The closure has to be a predicate (returning a bool value)
# > else `null` is returned
# > The closure also has to be valid for the types it receives
# > These will be flagged as errors later as closure annotations
# > are implemented
export def --examples=$find_examples find [
    fn: closure # the closure used to perform the search 
]: [
    list<any> -> any
] {
    filter {|e| try {do $fn $e} } | try { first }
}

const find_index_examples = [
    {
        description: ""
        example: r#'["iter", "abc", "shell", "around", "nushell", "std"] | iter find-index {|x| $x starts-with 's'}'#
        result: 2,
    }
    {
        description: ""
        example: r#'[3 5 13 91] | iter find-index {|x| $x mod 2 == 0}'#
        result: -1,
    }
]

# Returns the index of the first element that matches the predicate or
# -1 if none
#
# # Invariant
# > The closure has to return a bool
export def --examples=$find_index_examples find-index [
    fn: closure # the closure used to perform the search
]: [
    list<any> -> int
] {
    enumerate
    | find {|e| $e.item | do $fn $e.item }
    | try { get index } catch { -1 }
}

const intersperse_examples = [
    {
        description: "",
        example: r#'[1 2 3 4] | iter intersperse 0'#,
        result: [1 0 2 0 3 0 4],
    }
]

# Returns a new list with the separator between adjacent
# items of the original list
export def --examples=$intersperse_examples intersperse [
    separator: any # the separator to be used
]: [
    list<any> -> list<any>
] {
    reduce --fold [] {|e, acc|
         $acc ++ [$e, $separator]
    } 
    | match $in {
         [] => [],
         $xs => ($xs | take (($xs | length) - 1 ))
    }
}

const scan_examples = [
    {
        description: ""
        example: r#'[1 2 3] | iter scan 0 {|x, y| $x + $y}'#
        result: [0, 1, 3, 6]
    }
    {
        description: "use the `--noinit(-n)` flag to remove the initial value from the final result"
        example: r#'[1 2 3] | iter scan 0 {|x, y| $x + $y} -n'#
        result: [1, 3, 6]
    }
]

# Returns a list of intermediate steps performed by `reduce`
# (`fold`). It takes two arguments, an initial value to seed the
# initial state and a closure that takes two arguments, the first
# being the list element in the current iteration and the second
# the internal state.
# The internal state is also provided as pipeline input.
export def --examples=$scan_examples scan [ # -> list<any>
    init: any            # initial value to seed the initial state
    fn: closure          # the closure to perform the scan
    --noinit(-n)         # remove the initial value from the result
]: [
    list<any> -> list<any>
] {
    generate {|e, acc|
        let out = $acc | do $fn $e $acc
        {next: $out, out: $out}
    } $init
    | if not $noinit { prepend $init } else { }
}

const filter_map_examples = [
    {
        description: ""
        example: r#'[2 5 "4" 7] | iter filter-map {|e| $e ** 2}'#
        result: [4 25 49]
    }
]

# Returns a list of values for which the supplied closure does not
# return `null` or an error. It is equivalent to 
#     `$in | each $fn | filter $fn`
export def --examples=$filter_map_examples filter-map [
    fn: closure                # the closure to apply to the input
]: [
    list<any> -> list<any>
] {
    each {|$e|
        try {
            do $fn $e 
        } catch {
            null 
        }
    } 
    | filter {|e|
        $e != null
    }
}

const flat_map_examples = [
    {
        description: ""
        example: r#'[[1 2 3] [2 3 4] [5 6 7]] | iter flat-map {|e| $e | math sum}'#
        result: [6 9 18]
    }
]

# Maps a closure to each nested structure and flattens the result
export def --examples=$flat_map_examples flat-map [ # -> list<any>
    fn: closure              # the closure to map to the nested structures
]: [
    list<any> -> list<any>
] {
    each {|e| do $fn $e } | flatten
}

const zip_with_examples = [
    {
        description: ""
        example: r#'[1 2 3] | iter zip-with [2 3 4] {|a, b| $a + $b }'#
        result: [3 5 7]
    }
]

# Zips two structures and applies a closure to each of the zips
export def --examples=$zip_with_examples zip-with [ # -> list<any>
    other: any               # the structure to zip with
    fn: closure              # the closure to apply to the zips
] {
    zip $other 
    | each {|e|
        reduce {|e, acc| do $fn $acc $e }
    }
}

const zip_into_examples = [
    {
        description: ""
        example: r#'[1 2 3] | iter zip-into-record [2 3 4]'#
        result: [["1" "2" "3"]; [2 3 4]]
    }
]

# Zips two lists and returns a record with the first list as headers
export def --examples=$zip_into_examples zip-into-record [ # -> table<any>
    other: list                     # the values to zip with
] {
    zip $other
    | into record
    | [$in]
}
