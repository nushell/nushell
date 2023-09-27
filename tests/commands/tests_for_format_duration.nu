use std assert

# Parameter name:
# sig type   : duration
# name       : format value
# type       : positional
# shape      : string
# description: the unit in which to display the duration

# Parameter name:
# sig type   : list<duration>
# name       : format value
# type       : positional
# shape      : string
# description: the unit in which to display the duration

# Parameter name:
# sig type   : table
# name       : format value
# type       : positional
# shape      : string
# description: the unit in which to display the duration


# This is the custom command 1 for format_duration:

#[test]
def format_duration_convert_µs_duration_to_the_requested_second_duration_as_a_string_1 [] {
  let result = (1000000µs | format duration sec)
  assert ($result == 1 sec)
}

# This is the custom command 2 for format_duration:

#[test]
def format_duration_convert_durations_to_µs_duration_as_strings_2 [] {
  let result = ([1sec 2sec] | format duration µs)
  assert ($result == [1000000 µs, 2000000 µs])
}

# This is the custom command 3 for format_duration:

#[test]
def format_duration_convert_duration_to_µs_as_a_string_if_unit_asked_for_was_us_3 [] {
  let result = (1sec | format duration us)
  assert ($result == 1000000 µs)
}


