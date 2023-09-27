use std assert

# Parameter name:
# sig type   : list<any>
# name       : closure
# type       : positional
# shape      : closure(any, any, int)
# description: reducing function

# Parameter name:
# sig type   : list<any>
# name       : fold
# type       : named
# shape      : any
# description: reduce with initial value

# Parameter name:
# sig type   : range
# name       : closure
# type       : positional
# shape      : closure(any, any, int)
# description: reducing function

# Parameter name:
# sig type   : range
# name       : fold
# type       : named
# shape      : any
# description: reduce with initial value

# Parameter name:
# sig type   : table
# name       : closure
# type       : positional
# shape      : closure(any, any, int)
# description: reducing function

# Parameter name:
# sig type   : table
# name       : fold
# type       : named
# shape      : any
# description: reduce with initial value


# This is the custom command 1 for reduce:

#[test]
def reduce_sum_values_of_a_list_same_as_math_sum_1 [] {
  let result = ([ 1 2 3 4 ] | reduce {|it, acc| $it + $acc })
  assert ($result == 10)
}

# This is the custom command 2 for reduce:

#[test]
def reduce_sum_values_of_a_list_plus_their_indexes_2 [] {
  let result = ([ 8 7 6 ] | enumerate | reduce -f 0 {|it, acc| $acc + $it.item + $it.index })
  assert ($result == 24)
}

# This is the custom command 3 for reduce:

#[test]
def reduce_sum_values_with_a_starting_value_fold_3 [] {
  let result = ([ 1 2 3 4 ] | reduce -f 10 {|it, acc| $acc + $it })
  assert ($result == 20)
}

# This is the custom command 4 for reduce:

#[test]
def reduce_replace_selected_characters_in_a_string_with_x_4 [] {
  let result = ([ i o t ] | reduce -f "Arthur, King of the Britons" {|it, acc| $acc | str replace -a $it "X" })
  assert ($result == ArXhur, KXng Xf Xhe BrXXXns)
}

# This is the custom command 5 for reduce:

#[test]
def reduce_add_ascending_numbers_to_each_of_the_filenames_and_join_with_semicolons_5 [] {
  let result = (['foo.gz', 'bar.gz', 'baz.gz'] | enumerate | reduce -f '' {|str all| $"($all)(if $str.index != 0 {'; '})($str.index + 1)-($str.item)" })
  assert ($result == 1-foo.gz; 2-bar.gz; 3-baz.gz)
}

# This is the custom command 6 for reduce:

#[test]
def reduce_concatenate_a_string_with_itself_using_a_range_to_determine_the_number_of_times_6 [] {
  let result = (let s = "Str"; 0..2 | reduce -f '' {|it, acc| $acc + $s})
  assert ($result == StrStrStr)
}


