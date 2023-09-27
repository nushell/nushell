use std assert


# This is the custom command 1 for str_downcase:

#[test]
def str_downcase_downcase_contents_1 [] {
  let result = ('NU' | str downcase)
  assert ($result == nu)
}

# This is the custom command 2 for str_downcase:

#[test]
def str_downcase_downcase_contents_2 [] {
  let result = ('TESTa' | str downcase)
  assert ($result == testa)
}

# This is the custom command 3 for str_downcase:

#[test]
def str_downcase_downcase_contents_3 [] {
  let result = ([[ColA ColB]; [Test ABC]] | str downcase ColA)
  assert ($result == [{ColA: test, ColB: ABC}])
}

# This is the custom command 4 for str_downcase:

#[test]
def str_downcase_downcase_contents_4 [] {
  let result = ([[ColA ColB]; [Test ABC]] | str downcase ColA ColB)
  assert ($result == [{ColA: test, ColB: abc}])
}


