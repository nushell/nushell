use std assert

# Parameter name:
# sig type   : string
# name       : pattern
# type       : positional
# shape      : string
# description: the regular expression to use


# This is the custom command 1 for regex:

#[test]
def regex_parse_a_string_with_a_regular_expression_1 [] {
  let result = ("hello world" | regex '(?P<first>\w+) (?P<second>\w+)')
  assert ($result == [{input: hello world, capture_name: capgrp0, match: hello world, begin: 0, end: 11}, {input: hello world, capture_name: first, match: hello, begin: 0, end: 5}, {input: hello world, capture_name: second, match: world, begin: 6, end: 11}])
}


