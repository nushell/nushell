let file = '/dev/null'
^ls o> $file
^ls o> ($file | path expand)
^ls o> $"($file)"
^ls o> "/dev/null"
