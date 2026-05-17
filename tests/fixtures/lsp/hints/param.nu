def cmd [
  a1
  a2
  --flag (-f)
  a3? # arg3
  a4?
  ...arg_rest
] { }

ls | cmd 1 $nu -f (
  cmd 1
  2
) ...[(cmd 1 2)]
