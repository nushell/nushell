use std assert

# Parameter name:
# sig type   : string
# name       : skip
# type       : named
# shape      : int
# description: number of rows to skip before detecting

# Parameter name:
# sig type   : string
# name       : no-headers
# type       : switch
# shape      : 
# description: don't detect headers

# Parameter name:
# sig type   : string
# name       : combine-columns
# type       : named
# shape      : range
# description: columns to be combined; listed as a range


# This is the custom command 1 for detect_columns:

#[test]
def detect_columns_splits_string_across_multiple_columns_1 [] {
  let result = ('a b c' | detect columns -n)
  assert ($result == [{column0: a, column1: b, column2: c}])
}

# This is the custom command 2 for detect_columns:

#[test]
def detect_columns__2 [] {
  let result = ($'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns -c 0..1)
  assert ($result == )
}

# This is the custom command 3 for detect_columns:

#[test]
def detect_columns_splits_a_multi_line_string_into_columns_with_headers_detected_3 [] {
  let result = ($'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns -c -2..-1)
  assert ($result == )
}

# This is the custom command 4 for detect_columns:

#[test]
def detect_columns_splits_a_multi_line_string_into_columns_with_headers_detected_4 [] {
  let result = ($'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns -c 2..)
  assert ($result == )
}

# This is the custom command 5 for detect_columns:

#[test]
def detect_columns_parse_external_ls_command_and_combine_columns_for_datetime_5 [] {
  let result = (^ls -lh | detect columns --no-headers --skip 1 --combine-columns 5..7)
  assert ($result == )
}


