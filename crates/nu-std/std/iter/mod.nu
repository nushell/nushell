# | Filter Extensions
# 
# This module implements extensions to the `filters` commands.
#
# They are prefixed with `iter` so as to avoid conflicts with the inbuilt filters.

# Returns the first element of the list that matches the closure predicate, `null` otherwise
#
# # Invariant
#
# The closure must be a predicate (returning a bool value), otherwise `null` is returned.
# The closure also has to be valid for the types it receives.
# These will be flagged as errors later as closure annotations are implemented.
@example "Find an element starting with 'a'" {
    ["shell", "abc", "around", "nushell", "std"] | iter find {|e| $e starts-with "a" }
} --result "abc"
@example "Try to find an even element" { ["shell", "abc", "around", "nushell", "std"] | iter find {|e| $e mod 2 == 0} } --result null
export def find [
    fn: closure # the closure used to perform the search 
] {
    where {|e| try {do $fn $e} } | try { first }
}

# Returns the index of the first element that matches the predicate or -1 if none
#
# # Invariant
#
# The closure must return a bool
@example "Find the index of an element starting with 's'" {
    ["iter", "abc", "shell", "around", "nushell", "std"] | iter find-index {|x| $x starts-with 's'}
} --result 2
@example "Try to find the index of an even element" {
    [3 5 13 91] | iter find-index {|x| $x mod 2 == 0}
} --result -1
export def find-index [
    fn: closure # the closure used to perform the search
] {
    enumerate
    | find {|e| $e.item | do $fn $e.item }
    | try { get index } catch { -1 }
}

# Returns a new list with the separator between adjacent items of the original list
@example "Intersperse the list with `0`" {
    [1 2 3 4] | iter intersperse 0
} --result [1 0 2 0 3 0 4]
export def intersperse [
    separator: any # the separator to be used
] {
    reduce --fold [] {|e, acc|
         $acc ++ [$e, $separator]
    } 
    | match $in {
         [] => [],
         $xs => ($xs | take (($xs | length) - 1 ))
    }
}

# Returns a list of intermediate steps performed by `reduce` (`fold`).
#
# It takes two arguments:
# * an initial value to seed the initial state
# * a closure that takes two arguments
#   1. the list element in the current iteration
#   2. the internal state
#
# The internal state is also provided as pipeline input.
@example "Get a running sum of the input list" {
    [1 2 3] | iter scan 0 {|x, y| $x + $y}
} --result [0, 1, 3, 6]
@example "use the `--noinit(-n)` flag to remove the initial value from the final result" {
    [1 2 3] | iter scan 0 {|x, y| $x + $y} -n
} --result [1, 3, 6]
export def scan [ # -> list<any>
    init: any            # initial value to seed the initial state
    fn: closure          # the closure to perform the scan
    --noinit(-n)         # remove the initial value from the result
] {
    generate {|e, acc|
        let out = $acc | do $fn $e $acc
        {next: $out, out: $out}
    } $init
    | if not $noinit { prepend $init } else { }
}

# Returns a list of values for which the supplied closure does not return `null` or an error.
#
# This is equivalent to 
#
#     $in | each $fn | where $fn
@example "Get the squares of elements that can be squared" {
    [2 5 "4" 7] | iter filter-map {|e| $e ** 2}
} --result [4, 25, 49]
export def filter-map [
    fn: closure                # the closure to apply to the input
] {
    each {|$e|
        try {
            do $fn $e 
        } catch {
            null 
        }
    } 
    | where {|e|
        $e != null
    }
}

# Maps a closure to each nested structure and flattens the result
@example "Get the sums of list elements" {
    [[1 2 3] [2 3 4] [5 6 7]] | iter flat-map {|e| $e | math sum}
} --result [6, 9, 18]
export def flat-map [ # -> list<any>
    fn: closure              # the closure to map to the nested structures
] {
    each {|e| do $fn $e } | flatten
}

# Zips two structures and applies a closure to each of the zips
@example "Add two lists element-wise" {
    [1 2 3] | iter zip-with [2 3 4] {|a, b| $a + $b }
} --result [3, 5, 7]
export def  zip-with [ # -> list<any>
    other: any               # the structure to zip with
    fn: closure              # the closure to apply to the zips
] {
    zip $other 
    | each {|e|
        reduce {|e, acc| do $fn $acc $e }
    }
}

# Zips two lists and returns a record with the first list as headers
@example "Create record from two lists" {
    [1 2 3] | iter zip-into-record [2 3 4]
} --result [{1: 2, 2: 3, 3: 4}]
export def zip-into-record [ # -> table<any>
    other: list                     # the values to zip with
] {
    zip $other
    | into record
    | [$in]
}
