# A closure with a tail is no row condition either: it never closes.
[0 1 2] | where {|x| $x > 1}.a
