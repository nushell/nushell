use std assert

# Parameter name:
# sig type   : list<any>
# name       : reverse
# type       : switch
# shape      : 
# description: Sort in reverse order

# Parameter name:
# sig type   : list<any>
# name       : ignore-case
# type       : switch
# shape      : 
# description: Sort string-based data case-insensitively

# Parameter name:
# sig type   : list<any>
# name       : values
# type       : switch
# shape      : 
# description: If input is a single record, sort the record by values; ignored if input is not a single record

# Parameter name:
# sig type   : list<any>
# name       : natural
# type       : switch
# shape      : 
# description: Sort alphanumeric string-based values naturally (1, 9, 10, 99, 100, ...)

# Parameter name:
# sig type   : record
# name       : reverse
# type       : switch
# shape      : 
# description: Sort in reverse order

# Parameter name:
# sig type   : record
# name       : ignore-case
# type       : switch
# shape      : 
# description: Sort string-based data case-insensitively

# Parameter name:
# sig type   : record
# name       : values
# type       : switch
# shape      : 
# description: If input is a single record, sort the record by values; ignored if input is not a single record

# Parameter name:
# sig type   : record
# name       : natural
# type       : switch
# shape      : 
# description: Sort alphanumeric string-based values naturally (1, 9, 10, 99, 100, ...)


# This is the custom command 1 for sort:

#[test]
def sort_sort_the_list_by_increasing_value_1 [] {
  let result = ([2 0 1] | sort)
  assert ($result == [0, 1, 2])
}

# This is the custom command 2 for sort:

#[test]
def sort_sort_the_list_by_decreasing_value_2 [] {
  let result = ([2 0 1] | sort -r)
  assert ($result == [2, 1, 0])
}

# This is the custom command 3 for sort:

#[test]
def sort_sort_a_list_of_strings_3 [] {
  let result = ([betty amy sarah] | sort)
  assert ($result == [amy, betty, sarah])
}

# This is the custom command 4 for sort:

#[test]
def sort_sort_a_list_of_strings_in_reverse_4 [] {
  let result = ([betty amy sarah] | sort -r)
  assert ($result == [sarah, betty, amy])
}

# This is the custom command 5 for sort:

#[test]
def sort_sort_strings_case_insensitive_5 [] {
  let result = ([airplane Truck Car] | sort -i)
  assert ($result == [airplane, Car, Truck])
}

# This is the custom command 6 for sort:

#[test]
def sort_sort_strings_reversed_case_insensitive_6 [] {
  let result = ([airplane Truck Car] | sort -i -r)
  assert ($result == [Truck, Car, airplane])
}

# This is the custom command 7 for sort:

#[test]
def sort_sort_record_by_key_case_insensitive_7 [] {
  let result = ({b: 3, a: 4} | sort)
  assert ($result == {a: 4, b: 3})
}

# This is the custom command 8 for sort:

#[test]
def sort_sort_record_by_value_8 [] {
  let result = ({b: 4, a: 3, c:1} | sort -v)
  assert ($result == {c: 1, a: 3, b: 4})
}


