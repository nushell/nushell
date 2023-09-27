use std assert

# Parameter name:
# sig type   : nothing
# name       : columns
# type       : named
# shape      : list<string>
# description: Closure that gets called when the LazyRecord needs to list the available column names

# Parameter name:
# sig type   : nothing
# name       : get-value
# type       : named
# shape      : closure(string)
# description: Closure to call when a value needs to be produced on demand


# This is the custom command 1 for lazy_make:

#[test]
def lazy_make_create_a_lazy_record_1 [] {
  let result = (lazy make --columns ["haskell", "futures", "nushell"] --get-value { |lazything| $lazything + "!" })
  assert ($result == )
}

# This is the custom command 2 for lazy_make:

#[test]
def lazy_make_test_the_laziness_of_lazy_records_2 [] {
  let result = (lazy make -c ["hello"] -g { |key| print $"getting ($key)!"; $key | str upcase })
  assert ($result == )
}


