def completer [] { [] }
def --env --wrapped "my cmd" [
  a: int, # the a
  name: string@completer
  b?: string = "x"
  --flag(-f): int = 3
  -s
  --long (-l)
  ...rest: string
]: [int -> string, nothing -> nothing] {
  $a
}
