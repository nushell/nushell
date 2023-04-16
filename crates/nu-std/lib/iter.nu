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
export def "iter find" [        # -> any | null  
    predicate: closure   # the closure used to perform the search 
] {
    let list = (self collect)
    try {
       $list | filter $predicate | get 0?
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
    separator: any,      # the separator to be used
] {
    let list = (self collect)

    let len = ($list | length);
    if ($list | is-empty) {
        return $list
    }

    $list 
    | enumerate
    | reduce -f [] {|it, acc|
        if ($it.index == $len - 1) {
           $acc ++ [$it.item]
        } else {
            $acc ++ [$it.item, $separator]
        }
    }
}

# Accepts inputs from a pipeline and builds a list for the `iter *`
# commands to work with
def "self collect" [] {
    reduce -f [] {|it, acc|
        $acc ++ $it
    }
}
