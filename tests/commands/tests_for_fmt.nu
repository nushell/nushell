use std assert


# This is the custom command 1 for fmt:

#[test]
def fmt_get_a_record_containing_multiple_formats_for_the_number_42_1 [] {
  let result = (42 | fmt)
  assert ($result == {binary: 0b101010, debug: 42, display: 42, lowerexp: 4.2e1, lowerhex: 0x2a, octal: 0o52, upperexp: 4.2E1, upperhex: 0x2A})
}


