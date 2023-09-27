use std assert

# Parameter name:
# sig type   : nothing
# name       : output-format
# type       : named
# shape      : string
# description: prints dates in this format (defaults to %Y-%m-%d)

# Parameter name:
# sig type   : nothing
# name       : input-format
# type       : named
# shape      : string
# description: give argument dates in this format (defaults to %Y-%m-%d)

# Parameter name:
# sig type   : nothing
# name       : begin-date
# type       : named
# shape      : string
# description: beginning date range

# Parameter name:
# sig type   : nothing
# name       : end-date
# type       : named
# shape      : string
# description: ending date

# Parameter name:
# sig type   : nothing
# name       : increment
# type       : named
# shape      : int
# description: increment dates by this number

# Parameter name:
# sig type   : nothing
# name       : days
# type       : named
# shape      : int
# description: number of days to print

# Parameter name:
# sig type   : nothing
# name       : reverse
# type       : switch
# shape      : 
# description: print dates in reverse


# This is the custom command 1 for seq_date:

#[test]
def seq_date_print_the_next_10_days_in_yyyy_mm_dd_format_with_newline_separator_1 [] {
  let result = (seq date --days 10)
  assert ($result == )
}

# This is the custom command 2 for seq_date:

#[test]
def seq_date_print_the_previous_10_days_in_yyyy_mm_dd_format_with_newline_separator_2 [] {
  let result = (seq date --days 10 -r)
  assert ($result == )
}

# This is the custom command 3 for seq_date:

#[test]
def seq_date_print_the_previous_10_days_starting_today_in_mmddyyyy_format_with_newline_separator_3 [] {
  let result = (seq date --days 10 -o '%m/%d/%Y' -r)
  assert ($result == )
}

# This is the custom command 4 for seq_date:

#[test]
def seq_date_print_the_first_10_days_in_january_2020_4 [] {
  let result = (seq date -b '2020-01-01' -e '2020-01-10')
  assert ($result == [2020-01-01, 2020-01-02, 2020-01-03, 2020-01-04, 2020-01-05, 2020-01-06, 2020-01-07, 2020-01-08, 2020-01-09, 2020-01-10])
}

# This is the custom command 5 for seq_date:

#[test]
def seq_date_print_every_fifth_day_between_january_1st_2020_and_january_31st_2020_5 [] {
  let result = (seq date -b '2020-01-01' -e '2020-01-31' -n 5)
  assert ($result == [2020-01-01, 2020-01-06, 2020-01-11, 2020-01-16, 2020-01-21, 2020-01-26, 2020-01-31])
}


