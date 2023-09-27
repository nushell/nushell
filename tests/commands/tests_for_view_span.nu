use std assert

# Parameter name:
# sig type   : nothing
# name       : start
# type       : positional
# shape      : int
# description: start of the span

# Parameter name:
# sig type   : nothing
# name       : end
# type       : positional
# shape      : int
# description: end of the span


# This is the custom command 1 for view_span:

#[test]
def view_span_view_the_source_of_a_span_1_and_2_are_just_example_values_use_the_return_of_debug__r_to_get_the_actual_values_1 [] {
  let result = (some | pipeline | or | variable | debug -r; view span 1 2)
  assert ($result == )
}


