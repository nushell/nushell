use std assert

# Parameter name:
# sig type   : list<any>
# name       : pattern
# type       : positional
# shape      : string
# description: the pattern to match. Eg) "{foo}: {bar}"

# Parameter name:
# sig type   : list<any>
# name       : regex
# type       : switch
# shape      : 
# description: use full regex syntax for patterns

# Parameter name:
# sig type   : string
# name       : pattern
# type       : positional
# shape      : string
# description: the pattern to match. Eg) "{foo}: {bar}"

# Parameter name:
# sig type   : string
# name       : regex
# type       : switch
# shape      : 
# description: use full regex syntax for patterns


# This is the custom command 1 for parse:

#[test]
def parse_parse_a_string_into_two_named_columns_1 [] {
  let result = ("hi there" | parse "{foo} {bar}")
  assert ($result == [{foo: hi, bar: there}])
}

# This is the custom command 2 for parse:

#[test]
def parse_parse_a_string_using_regex_pattern_2 [] {
  let result = ("hi there" | parse -r '(?P<foo>\w+) (?P<bar>\w+)')
  assert ($result == [{foo: hi, bar: there}])
}

# This is the custom command 3 for parse:

#[test]
def parse_parse_a_string_using_fancy_regex_named_capture_group_pattern_3 [] {
  let result = ("foo bar." | parse -r '\s*(?<name>\w+)(?=\.)')
  assert ($result == [{name: bar}])
}

# This is the custom command 4 for parse:

#[test]
def parse_parse_a_string_using_fancy_regex_capture_group_pattern_4 [] {
  let result = ("foo! bar." | parse -r '(\w+)(?=\.)|(\w+)(?=!)')
  assert ($result == [{capture0: , capture1: foo}, {capture0: bar, capture1: }])
}

# This is the custom command 5 for parse:

#[test]
def parse_parse_a_string_using_fancy_regex_look_behind_pattern_5 [] {
  let result = (" @another(foo bar)   " | parse -r '\s*(?<=[() ])(@\w+)(\([^)]*\))?\s*')
  assert ($result == [{capture0: @another, capture1: (foo bar)}])
}

# This is the custom command 6 for parse:

#[test]
def parse_parse_a_string_using_fancy_regex_look_ahead_atomic_group_pattern_6 [] {
  let result = ("abcd" | parse -r '^a(bc(?=d)|b)cd$')
  assert ($result == [{capture0: b}])
}


