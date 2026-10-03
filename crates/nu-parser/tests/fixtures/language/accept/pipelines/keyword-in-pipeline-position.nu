[1 2 3] | each { |x| if $x > 1 { $x } else { 0 } } | where $it > 0
