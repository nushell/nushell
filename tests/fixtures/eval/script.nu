def main [] {
  somefunc "foo"
}

def somefunc [
  somearg: unknown_type
] {
  echo $somearg
}
