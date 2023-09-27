use std assert

# Parameter name:
# sig type   : list<string>
# name       : path
# type       : switch
# shape      : 
# description: Replaces all backslashes with double backslashes, useful for Path.

# Parameter name:
# sig type   : string
# name       : path
# type       : switch
# shape      : 
# description: Replaces all backslashes with double backslashes, useful for Path.


# This is the custom command 1 for str_expand:

#[test]
def str_expand_define_a_range_inside_braces_to_produce_a_list_of_string_1 [] {
  let result = ("{3..5}" | str expand)
  assert ($result == [3, 4, 5])
}

# This is the custom command 2 for str_expand:

#[test]
def str_expand_ignore_the_next_character_after_the_backslash__2 [] {
  let result = ('A{B\,,C}' | str expand)
  assert ($result == [AB,, AC])
}

# This is the custom command 3 for str_expand:

#[test]
def str_expand_commas_that_are_not_inside_any_braces_need_to_be_skipped_3 [] {
  let result = ('Welcome\, {home,mon ami}!' | str expand)
  assert ($result == [Welcome, home!, Welcome, mon ami!])
}

# This is the custom command 4 for str_expand:

#[test]
def str_expand_use_double_backslashes_to_add_a_backslash_4 [] {
  let result = ('A{B\\,C}' | str expand)
  assert ($result == [AB\, AC])
}

# This is the custom command 5 for str_expand:

#[test]
def str_expand_export_comma_separated_values_inside_braces__to_a_string_list_5 [] {
  let result = ("{apple,banana,cherry}" | str expand)
  assert ($result == [apple, banana, cherry])
}

# This is the custom command 6 for str_expand:

#[test]
def str_expand_if_the_piped_data_is_path_you_may_want_to_use___path_flag_or_else_manually_replace_the_backslashes_with_double_backslashes_6 [] {
  let result = ('C:\{Users,Windows}' | str expand --path)
  assert ($result == [C:\Users, C:\Windows])
}

# This is the custom command 7 for str_expand:

#[test]
def str_expand_brace_expressions_can_be_used_one_after_another_7 [] {
  let result = ("A{b,c}D{e,f}G" | str expand)
  assert ($result == [AbDeG, AbDfG, AcDeG, AcDfG])
}

# This is the custom command 8 for str_expand:

#[test]
def str_expand_collection_may_include_an_empty_item_it_can_be_put_at_the_start_of_the_list_8 [] {
  let result = ("A{,B,C}" | str expand)
  assert ($result == [A, AB, AC])
}

# This is the custom command 9 for str_expand:

#[test]
def str_expand_empty_item_can_be_at_the_end_of_the_collection_9 [] {
  let result = ("A{B,C,}" | str expand)
  assert ($result == [AB, AC, A])
}

# This is the custom command 10 for str_expand:

#[test]
def str_expand_empty_item_can_be_in_the_middle_of_the_collection_10 [] {
  let result = ("A{B,,C}" | str expand)
  assert ($result == [AB, A, AC])
}

# This is the custom command 11 for str_expand:

#[test]
def str_expand_also_it_is_possible_to_use_one_inside_another_here_is_a_real_world_example_that_creates_files_11 [] {
  let result = ("A{B{1,3},C{2,5}}D" | str expand)
  assert ($result == [AB1D, AB3D, AC2D, AC5D])
}


