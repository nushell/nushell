use std assert

# Parameter name:
# sig type   : record
# name       : pretty
# type       : named
# shape      : int
# description: Formats the XML text with the provided indentation setting


# This is the custom command 1 for to_xml:

#[test]
def to_xml_outputs_an_xml_string_representing_the_contents_of_this_table_1 [] {
  let result = ({tag: note attributes: {} content : [{tag: remember attributes: {} content : [{tag: null attrs: null content : Event}]}]} | to xml)
  assert ($result == <note><remember>Event</remember></note>)
}

# This is the custom command 2 for to_xml:

#[test]
def to_xml_when_formatting_xml_null_and_empty_record_fields_can_be_omitted_and_strings_can_be_written_without_a_wrapping_record_2 [] {
  let result = ({tag: note content : [{tag: remember content : [Event]}]} | to xml)
  assert ($result == <note><remember>Event</remember></note>)
}

# This is the custom command 3 for to_xml:

#[test]
def to_xml_optionally_formats_the_text_with_a_custom_indentation_setting_3 [] {
  let result = ({tag: note content : [{tag: remember content : [Event]}]} | to xml -p 3)
  assert ($result == <note>
   <remember>Event</remember>
</note>)
}


