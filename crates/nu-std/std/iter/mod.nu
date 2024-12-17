# | Filter Extensions
# 
# This module implements extensions to the `filters` commands
#
# They are prefixed with `iter` so as to avoid conflicts with 
# the inbuilt filters

# Returns the first element of the list that matches the
# closure predicate, `null` otherwise
#
# # Invariant
# > The closure has to be a predicate (returning a bool value)
# > else `null` is returned
# > The closure also has to be valid for the types it receives
# > These will be flagged as errors later as closure annotations
# > are implemented
#
# # Example
# ```
# use std ["assert equal" "iter find"]
#
# let haystack = ["shell", "abc", "around", "nushell", "std"] 
#
# let found = ($haystack | iter find {|it| $it starts-with "a" })
# let not_found = ($haystack | iter find {|it| $it mod 2 == 0})
# 
# assert equal $found "abc"
# assert equal $not_found null
# ```
export def find [ # -> any | null  
    fn: closure          # the closure used to perform the search 
] {
    try {
       filter $fn | get 0?
    } catch {
       null
    }
}

# Returns the index of the first element that matches the predicate or
# -1 if none
#
# # Invariant
# > The closure has to return a bool
#
# # Example
# ```nu
# use std ["assert equal" "iter find-index"]
#
# let res = (
#     ["iter", "abc", "shell", "around", "nushell", "std"]
#     | iter find-index {|x| $x starts-with 's'}
# )
# assert equal $res 2
#
# let is_even = {|x| $x mod 2 == 0}
# let res = ([3 5 13 91] | iter find-index $is_even)
# assert equal $res -1
# ```
export def find-index [ # -> int
    fn: closure                # the closure used to perform the search
] {
    let matches = (
        enumerate
        | each {|it|
            if (do $fn $it.item) {
                $it.index
            }
        }
    )

    if ($matches | is-empty) {
        -1
    } else {
        $matches | first
    }
}

# Returns a new list with the separator between adjacent
# items of the original list
#
# # Example
# ```
# use std ["assert equal" "iter intersperse"]
#
# let res = ([1 2 3 4] | iter intersperse 0)
# assert equal $res [1 0 2 0 3 0 4]
# ```
export def intersperse [ # -> list<any>
    separator: any              # the separator to be used
] {
    reduce --fold [] {|it, acc|
         $acc ++ [$it, $separator]
    } 
    | match $in {
         [] => [],
         $xs => ($xs | take (($xs | length) - 1 ))
    }
}

# Returns a list of intermediate steps performed by `reduce`
# (`fold`). It takes two arguments, an initial value to seed the
# initial state and a closure that takes two arguments, the first
# being the list element in the current iteration and the second
# the internal state.
# The internal state is also provided as pipeline input.
#
# # Example
# ```
# use std ["assert equal" "iter scan"]
# let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y})
#
# assert equal $scanned [0, 1, 3, 6]
#
# # use the --noinit(-n) flag to remove the initial value from
# # the final result
# let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y} -n)
#
# assert equal $scanned [1, 3, 6]
# ```
export def scan [ # -> list<any>
    init: any            # initial value to seed the initial state
    fn: closure          # the closure to perform the scan
    --noinit(-n)         # remove the initial value from the result
] {                      
    reduce --fold [$init] {|it, acc|
        let acc_last = $acc | last
        $acc ++ [($acc_last | do $fn $it $acc_last)]
    }
    | if $noinit {
        $in | skip
    } else {
        $in
    }
}

# Returns a list of values for which the supplied closure does not
# return `null` or an error. It is equivalent to 
#     `$in | each $fn | filter $fn`
#
# # Example
# ```nu
# use std ["assert equal" "iter filter-map"]
#
# let res = ([2 5 "4" 7] | iter filter-map {|it| $it ** 2})
#
# assert equal $res [4 25 49]
# ```
export def filter-map [ # -> list<any>
    fn: closure                # the closure to apply to the input
] {
    each {|$it|
        try {
            do $fn $it 
        } catch {
            null 
        }
    } 
    | filter {|it|
        $it != null
    }
}

# Maps a closure to each nested structure and flattens the result
#
# # Example
# ```nu
# use std ["assert equal" "iter flat-map"]
#
# let res = (
#     [[1 2 3] [2 3 4] [5 6 7]] | iter flat-map {|it| $it | math sum}
# )
# assert equal $res [6 9 18]
# ```
export def flat-map [ # -> list<any>
    fn: closure              # the closure to map to the nested structures
] {
    each {|it| do $fn $it } | flatten
}

# Zips two structures and applies a closure to each of the zips
#
# # Example
# ```nu
# use std ["assert equal" "iter iter zip-with"]
#
# let res = (
#     [1 2 3] | iter zip-with [2 3 4] {|a, b| $a + $b }
# )
#
# assert equal $res [3 5 7]
# ```
export def zip-with [ # -> list<any>
    other: any               # the structure to zip with
    fn: closure              # the closure to apply to the zips
] {
    zip $other 
    | each {|it|
        reduce {|it, acc| do $fn $acc $it }
    }
}

# Zips two lists and returns a record with the first list as headers
#
# # Example
# ```nu
# use std ["assert equal" "iter iter zip-into-record"]
#
# let res = (
#     [1 2 3] | iter zip-into-record [2 3 4]
# )
#
# assert equal $res [
#     [1 2 3];
#     [2 3 4]
# ]
# ```
export def zip-into-record [ # -> table<any>
    other: list                     # the values to zip with
] {
    zip $other
    | into record
    | [$in]
}
