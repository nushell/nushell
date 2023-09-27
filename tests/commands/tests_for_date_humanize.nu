use std assert


# This is the custom command 1 for date_humanize:

#[test]
def date_humanize_print_a_humanized_format_for_the_date_relative_to_now_1 [] {
  let result = ("2021-10-22 20:00:12 +01:00" | date humanize)
  assert ($result == )
}


