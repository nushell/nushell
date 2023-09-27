use std assert


# This is the custom command 1 for date_to-table:

#[test]
def date_to-table_convert_the_current_date_into_a_table_1 [] {
  let result = (date to-table)
  assert ($result == )
}

# This is the custom command 2 for date_to-table:

#[test]
def date_to-table_convert_the_date_into_a_table_2 [] {
  let result = (date now | date to-table)
  assert ($result == )
}

# This is the custom command 3 for date_to-table:

#[test]
def date_to-table_convert_a_given_date_into_a_table_3 [] {
  let result = (2020-04-12T22:10:57.000000789+02:00 | date to-table)
  assert ($result == [{year: 2020, month: 4, day: 12, hour: 22, minute: 10, second: 57, nanosecond: 789, timezone: "+02:00"}])
}

# This is the custom command 4 for date_to-table:

#[test]
def date_to-table_convert_a_given_date_into_a_table_4 [] {
  let result = ('2020-04-12 22:10:57 +0200' | into datetime | date to-table)
  assert ($result == [{year: 2020, month: 4, day: 12, hour: 22, minute: 10, second: 57, nanosecond: 0, timezone: "+02:00"}])
}


