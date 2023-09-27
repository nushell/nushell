use std assert

# Parameter name:
# sig type   : list<any>
# name       : regex
# type       : named
# shape      : string
# description: regex to match with

# Parameter name:
# sig type   : list<any>
# name       : ignore-case
# type       : switch
# shape      : 
# description: case-insensitive regex mode; equivalent to (?i)

# Parameter name:
# sig type   : list<any>
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode: ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : list<any>
# name       : dotall
# type       : switch
# shape      : 
# description: dotall regex mode: allow a dot . to match newlines \n; equivalent to (?s)

# Parameter name:
# sig type   : list<any>
# name       : columns
# type       : named
# shape      : list<string>
# description: column names to be searched (with rest parameter, not regex yet)

# Parameter name:
# sig type   : list<any>
# name       : invert
# type       : switch
# shape      : 
# description: invert the match

# Parameter name:
# sig type   : string
# name       : regex
# type       : named
# shape      : string
# description: regex to match with

# Parameter name:
# sig type   : string
# name       : ignore-case
# type       : switch
# shape      : 
# description: case-insensitive regex mode; equivalent to (?i)

# Parameter name:
# sig type   : string
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode: ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : string
# name       : dotall
# type       : switch
# shape      : 
# description: dotall regex mode: allow a dot . to match newlines \n; equivalent to (?s)

# Parameter name:
# sig type   : string
# name       : columns
# type       : named
# shape      : list<string>
# description: column names to be searched (with rest parameter, not regex yet)

# Parameter name:
# sig type   : string
# name       : invert
# type       : switch
# shape      : 
# description: invert the match

# Parameter name:
# sig type   : table
# name       : regex
# type       : named
# shape      : string
# description: regex to match with

# Parameter name:
# sig type   : table
# name       : ignore-case
# type       : switch
# shape      : 
# description: case-insensitive regex mode; equivalent to (?i)

# Parameter name:
# sig type   : table
# name       : multiline
# type       : switch
# shape      : 
# description: multi-line regex mode: ^ and $ match begin/end of line; equivalent to (?m)

# Parameter name:
# sig type   : table
# name       : dotall
# type       : switch
# shape      : 
# description: dotall regex mode: allow a dot . to match newlines \n; equivalent to (?s)

# Parameter name:
# sig type   : table
# name       : columns
# type       : named
# shape      : list<string>
# description: column names to be searched (with rest parameter, not regex yet)

# Parameter name:
# sig type   : table
# name       : invert
# type       : switch
# shape      : 
# description: invert the match


# This is the custom command 1 for find:

#[test]
def find_search_for_multiple_terms_in_a_command_output_1 [] {
  let result = (ls | find toml md sh)
  assert ($result == )
}

# This is the custom command 2 for find:

#[test]
def find_search_for_a_term_in_a_string_2 [] {
  let result = ('Cargo.toml' | find toml)
  assert ($result == Cargo.toml)
}

# This is the custom command 3 for find:

#[test]
def find_search_a_number_or_a_file_size_in_a_list_of_numbers_3 [] {
  let result = ([1 5 3kb 4 3Mb] | find 5 3kb)
  assert ($result == [5, 2.9 KiB])
}

# This is the custom command 4 for find:

#[test]
def find_search_a_char_in_a_list_of_string_4 [] {
  let result = ([moe larry curly] | find l)
  assert ($result == [larry, curly])
}

# This is the custom command 5 for find:

#[test]
def find_find_using_regex_5 [] {
  let result = ([abc bde arc abf] | find --regex "ab")
  assert ($result == [abc, abf])
}

# This is the custom command 6 for find:

#[test]
def find_find_using_regex_case_insensitive_6 [] {
  let result = ([aBc bde Arc abf] | find --regex "ab" -i)
  assert ($result == [aBc, abf])
}

# This is the custom command 7 for find:

#[test]
def find_find_value_in_records_using_regex_7 [] {
  let result = ([[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find -r "nu")
  assert ($result == [{version: 0.1.0, name: nushell}])
}

# This is the custom command 8 for find:

#[test]
def find_find_inverted_values_in_records_using_regex_8 [] {
  let result = ([[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find -r "nu" --invert)
  assert ($result == [{version: 0.1.1, name: fish}, {version: 0.2.0, name: zsh}])
}

# This is the custom command 9 for find:

#[test]
def find_find_value_in_list_using_regex_9 [] {
  let result = ([["Larry", "Moe"], ["Victor", "Marina"]] | find -r "rr")
  assert ($result == [[Larry, Moe]])
}

# This is the custom command 10 for find:

#[test]
def find_find_inverted_values_in_records_using_regex_10 [] {
  let result = ([["Larry", "Moe"], ["Victor", "Marina"]] | find -r "rr" --invert)
  assert ($result == [[Victor, Marina]])
}

# This is the custom command 11 for find:

#[test]
def find_remove_ansi_sequences_from_result_11 [] {
  let result = ([[foo bar]; [abc 123] [def 456]] | find 123 | get bar | ansi strip)
  assert ($result == )
}

# This is the custom command 12 for find:

#[test]
def find_find_and_highlight_text_in_specific_columns_12 [] {
  let result = ([[col1 col2 col3]; [moe larry curly] [larry curly moe]] | find moe -c [col1])
  assert ($result == [{col1: [37m[0m[41;37mmoe[0m[37m[0m, col2: larry, col3: curly}])
}


