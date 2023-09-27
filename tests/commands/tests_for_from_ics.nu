use std assert


# This is the custom command 1 for from_ics:

#[test]
def from_ics_converts_ics_formatted_string_to_table_1 [] {
  let result = ('BEGIN:VCALENDAR
            END:VCALENDAR' | from ics)
  assert ($result == [{properties: [], events: [], alarms: [], to-Dos: [], journals: [], free-busys: [], timezones: []}])
}


