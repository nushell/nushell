(ls | where (($in | length) > 0) | each { |f| ($f.name | str length) + (1 | into int) }) | math sum
