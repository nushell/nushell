use std assert

# Parameter name:
# sig type   : string
# name       : keep-comments
# type       : switch
# shape      : 
# description: add comment nodes to result

# Parameter name:
# sig type   : string
# name       : keep-pi
# type       : switch
# shape      : 
# description: add processing instruction nodes to result


# This is the custom command 1 for from_xml:

#[test]
def from_xml_converts_xml_formatted_string_to_record_1 [] {
  let result = ('<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml)
  assert ($result == {tag: note, attributes: {}, content: [{tag: remember, attributes: {}, content: [{tag: , attributes: , content: Event}]}]})
}


