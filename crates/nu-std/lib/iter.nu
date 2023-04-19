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
export def "iter find" [ # -> any | null  
    fn: closure          # the closure used to perform the search 
] {
    try {
       filter $fn | get 0?
    } catch {
       null
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
export def "iter intersperse" [ # -> list<any>
    separator: any,             # the separator to be used
] {
    reduce -f [] {|it, acc|
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
# being the internal state and the second the list element in the
# current iteration.
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
export def "iter scan" [ # -> list<any>
    init: any            # initial value to seed the initial state
    fn: closure          # the closure to perform the scan
    --noinit(-n)         # remove the initial value from the result
] {                      
   reduce -f [$init] {|it, acc|
      $acc ++ [(do $fn ($acc | last) $it)]
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
export def "iter filter-map" [ # -> list<any>
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
