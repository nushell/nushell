use std assert

# Parameter name:
# sig type   : list<any>
# name       : column-name
# type       : positional
# shape      : string
# description: column name to calc frequency, no need to provide if input is just a list

# Parameter name:
# sig type   : list<any>
# name       : frequency-column-name
# type       : positional
# shape      : string
# description: histogram's frequency column, default to be frequency column output

# Parameter name:
# sig type   : list<any>
# name       : percentage-type
# type       : named
# shape      : string
# description: percentage calculate method, can be 'normalize' or 'relative', in 'normalize', defaults to be 'normalize'


# This is the custom command 1 for histogram:

#[test]
def histogram_compute_a_histogram_of_file_types_1 [] {
  let result = (ls | histogram type)
  assert ($result == )
}

# This is the custom command 2 for histogram:

#[test]
def histogram_compute_a_histogram_for_the_types_of_files_with_frequency_column_named_freq_2 [] {
  let result = (ls | histogram type freq)
  assert ($result == )
}

# This is the custom command 3 for histogram:

#[test]
def histogram_compute_a_histogram_for_a_list_of_numbers_3 [] {
  let result = ([1 2 1] | histogram)
  assert ($result == [{value: 1, count: 2, quantile: 0.6666666666666666, percentage: 66.67%, frequency: ******************************************************************}, {value: 2, count: 1, quantile: 0.3333333333333333, percentage: 33.33%, frequency: *********************************}])
}

# This is the custom command 4 for histogram:

#[test]
def histogram_compute_a_histogram_for_a_list_of_numbers_and_percentage_is_based_on_the_maximum_value_4 [] {
  let result = ([1 2 3 1 1 1 2 2 1 1] | histogram --percentage-type relative)
  assert ($result == )
}


