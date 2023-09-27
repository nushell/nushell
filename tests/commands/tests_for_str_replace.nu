use std assert

# Parameter name:
# sig type   : list<string>
# name       : find
# type       : positional
# shape      : string
# description: the pattern to find

# Parameter name:
# sig type   : list<string>
# name       : replace
# type       : positional
# shape      : string
# description: the replacement string

# Parameter name:
# sig type   : list<string>
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of the pattern

# Parameter name:
# sig type   : list<string>
# name       : no-expand
# type       : switch
# shape      : 
# description: do not expand capture groups (like $name) in the replacement string

# Parameter name:
# sig type   : list<string>
# name       : regex
# type       : switch
# shape      : 
# description: match the pattern as a regular expression in the input, instead of a substring

# Parameter name:
# sig type   : list<string>
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode (implies --regex): ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : record
# name       : find
# type       : positional
# shape      : string
# description: the pattern to find

# Parameter name:
# sig type   : record
# name       : replace
# type       : positional
# shape      : string
# description: the replacement string

# Parameter name:
# sig type   : record
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of the pattern

# Parameter name:
# sig type   : record
# name       : no-expand
# type       : switch
# shape      : 
# description: do not expand capture groups (like $name) in the replacement string

# Parameter name:
# sig type   : record
# name       : regex
# type       : switch
# shape      : 
# description: match the pattern as a regular expression in the input, instead of a substring

# Parameter name:
# sig type   : record
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode (implies --regex): ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : string
# name       : find
# type       : positional
# shape      : string
# description: the pattern to find

# Parameter name:
# sig type   : string
# name       : replace
# type       : positional
# shape      : string
# description: the replacement string

# Parameter name:
# sig type   : string
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of the pattern

# Parameter name:
# sig type   : string
# name       : no-expand
# type       : switch
# shape      : 
# description: do not expand capture groups (like $name) in the replacement string

# Parameter name:
# sig type   : string
# name       : regex
# type       : switch
# shape      : 
# description: match the pattern as a regular expression in the input, instead of a substring

# Parameter name:
# sig type   : string
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode (implies --regex): ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : table
# name       : find
# type       : positional
# shape      : string
# description: the pattern to find

# Parameter name:
# sig type   : table
# name       : replace
# type       : positional
# shape      : string
# description: the replacement string

# Parameter name:
# sig type   : table
# name       : all
# type       : switch
# shape      : 
# description: replace all occurrences of the pattern

# Parameter name:
# sig type   : table
# name       : no-expand
# type       : switch
# shape      : 
# description: do not expand capture groups (like $name) in the replacement string

# Parameter name:
# sig type   : table
# name       : regex
# type       : switch
# shape      : 
# description: match the pattern as a regular expression in the input, instead of a substring

# Parameter name:
# sig type   : table
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode (implies --regex): ^ and $ match begin/end of line; equivalent to (?m)


# This is the custom command 1 for str_replace:

#[test]
def str_replace_find_and_replace_the_first_occurrence_of_a_substring_1 [] {
  let result = ('c:\some\cool\path' | str replace 'c:\some\cool' '~')
  assert ($result == ~\path)
}

# This is the custom command 2 for str_replace:

#[test]
def str_replace_find_and_replace_all_occurrences_of_a_substring_2 [] {
  let result = ('abc abc abc' | str replace -a 'b' 'z')
  assert ($result == azc azc azc)
}

# This is the custom command 3 for str_replace:

#[test]
def str_replace_find_and_replace_contents_with_capture_group_using_regular_expression_3 [] {
  let result = ('my_library.rb' | str replace -r '(.+).rb' '$1.nu')
  assert ($result == my_library.nu)
}

# This is the custom command 4 for str_replace:

#[test]
def str_replace_find_and_replace_all_occurrences_of_find_string_using_regular_expression_4 [] {
  let result = ('abc abc abc' | str replace -ar 'b' 'z')
  assert ($result == azc azc azc)
}

# This is the custom command 5 for str_replace:

#[test]
def str_replace_find_and_replace_all_occurrences_of_find_string_in_table_using_regular_expression_5 [] {
  let result = ([[ColA ColB ColC]; [abc abc ads]] | str replace -ar 'b' 'z' ColA ColC)
  assert ($result == [{ColA: azc, ColB: abc, ColC: ads}])
}

# This is the custom command 6 for str_replace:

#[test]
def str_replace_find_and_replace_all_occurrences_of_find_string_in_record_using_regular_expression_6 [] {
  let result = ({ KeyA: abc, KeyB: abc, KeyC: ads } | str replace -ar 'b' 'z' KeyA KeyC)
  assert ($result == {KeyA: azc, KeyB: abc, KeyC: ads})
}

# This is the custom command 7 for str_replace:

#[test]
def str_replace_find_and_replace_contents_without_using_the_replace_parameter_as_a_regular_expression_7 [] {
  let result = ('dogs_$1_cats' | str replace -r '\$1' '$2' -n)
  assert ($result == dogs_$2_cats)
}

# This is the custom command 8 for str_replace:

#[test]
def str_replace_use_captures_to_manipulate_the_input_text_using_regular_expression_8 [] {
  let result = ("abc-def" | str replace -r "(.+)-(.+)" "${2}_${1}")
  assert ($result == def_abc)
}

# This is the custom command 9 for str_replace:

#[test]
def str_replace_find_and_replace_with_fancy_regex_using_regular_expression_9 [] {
  let result = ('a successful b' | str replace -r '\b([sS])uc(?:cs|s?)e(ed(?:ed|ing|s?)|ss(?:es|ful(?:ly)?|i(?:ons?|ve(?:ly)?)|ors?)?)\b' '${1}ucce$2')
  assert ($result == a successful b)
}

# This is the custom command 10 for str_replace:

#[test]
def str_replace_find_and_replace_with_fancy_regex_using_regular_expression_10 [] {
  let result = ('GHIKK-9+*' | str replace -r '[*[:xdigit:]+]' 'z')
  assert ($result == GHIKK-z+*)
}

# This is the custom command 11 for str_replace:

#[test]
def str_replace_find_and_replace_on_individual_lines_using_multiline_regular_expression_11 [] {
  let result = ("non-matching line\n123. one line\n124. another line\n" | str replace -am '^[0-9]+\. ' '')
  assert ($result == non-matching line
one line
another line
)
}


