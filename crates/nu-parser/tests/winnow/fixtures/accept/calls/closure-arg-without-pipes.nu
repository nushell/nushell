[1 2 3] | each { $in * 2 } | reduce { |it, acc| $it + $acc }
