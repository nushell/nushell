let {$name, size: $bytes} = {name: a, size: 1}
print $name $bytes
match [1 2] { [$x, ..$rest] => [$x $rest] }
[{a: 1}] | each {|{$a}| $a + 1 }
let {$port = 80} = {}
print $port
mut m = 0
{$m} = {m: 1}
[$m] = [2]
