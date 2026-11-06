ls
  # filter
  | where size > 1kb
  | # keep names
  | get name
