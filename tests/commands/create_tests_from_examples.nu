def main [] {
  let command_list = (
    scope commands |
    where is_builtin == true and
    name != 'error make' and # these are commands that break things right now
    name != 'exit' and # some are reasonable
    name != 'alias' and
    name != 'ast' and
    name != 'break' and # some are reasonable
    name != 'cal' and
    name != 'cd' and
    name != 'clear' and # some are reasonable
    name != 'config reset' and # some are reasonable
    name != 'config' and
    name != 'continue' and # some are reasonable
    name != 'date' and
    name !~ 'dfr' |
    select name signatures examples
  )

  for command in $command_list {
    let command_name = $command.name | str replace -a ' ' '_'
    let signatures = $command | get signatures
    let parsed_signatures = parse_signatures $command_name $signatures
    let examples = $command | get examples
    let parsed_examples = parse_examples $command_name $examples

    let out_text = (
      "use std assert\n\n" +
      $"($parsed_signatures)\n" +
      $"($parsed_examples)\n"
    )

    let output_filename = $"tests_for_($command_name).nu"
    echo $out_text | save -f $output_filename
  }
}

# This command parses all the command signatures so that we can put
# a comment in the test script that has all the parameters that need
# to be tested.
def parse_signatures [command_name, signatures] {
  print $command_name
  mut out_text = ""
  let tabelized = ($signatures | transpose type sig)
  for signature in $tabelized {
    # print "This is the signature"
    # print $signature
    # print $"Working on signature type: ($signature.type)"
    for params in $signature.sig {
      # print $params
      for p in $params {
        if not ($p.parameter_name | is-empty) {
          # print "This is the parameter that isn't empty"
          # print $p
          $out_text = $out_text + "# Parameter name:\n"
          $out_text = $out_text +  $"# sig type   : ($signature.type)\n"
          $out_text = $out_text +  $"# name       : ($p.parameter_name)\n"
          $out_text = $out_text +  $"# type       : ($p.parameter_type)\n"
          $out_text = $out_text +  $"# shape      : ($p.syntax_shape)\n"
          $out_text = $out_text +  $'# description: ($p.description | str replace -a "\n" " ")'
          $out_text = $out_text +  "\n\n"
          # print $out_text
        }
      }
    }
  }

  $out_text
}

# This custom command parses all the examples and creates a custom
# command for each one. This is so that we can test each example in
# another way, with the stdlib testing.
def parse_examples [command_name, examples] {
  # print $examples
  mut example_count = 0
  mut out_text = ""
  for example in $examples {
    # print $"This is an example: ($example)"
    let description_text = (
      $example | get description | str trim | str downcase |
      str replace -a " " "_" |
      str replace -a "-" "_" |
      str replace -a -r '[\W]' "" # This regex says replace all non-alphanumeric characters with nothing
      # str replace -a '"' '' |
      # str replace -a "'" '' |
      # str replace -a '(' '' |
      # str replace -a ')' '' |
      # str replace -a '-' '' |
      # str replace -a '!' '' |
      # str replace -a '\' '' |
      # str replace -a '[' '' |
      # str replace -a ']' '' |
      # str replace -a ',' '' |
      # str replace -a '`' '' |
      # str replace -a '+' '' |
      # str replace -a ':' '' |
      # str replace -a '.' ''
    )
    let example_text = $example | get example
    # print $"get_result data type: ($example | get result | default "" | describe)"
    let example_result =  $example | get result
    let example_result_datatype = $example_result | describe
    let example_result_output = (
      if ($example_result_datatype == "string") {
        # print $"  This is a string: [($example_result)]"
        # print $'  ($example_result | str replace -r -a "\e\\[" `\e[`)'
        if $command_name == 'ansi_link' {
          $example_result | str replace -r -a "\e\\]" '\e]' | str replace -r -a "\e\\\\" '\e\\'
        } else {
          $example_result | str replace -r -a "\e\\[" '\e['
        }
      } else {
        # print '  name does not contain ansi'
        # print $"  This is not a string: [($example_result)] datatype: ($example_result_datatype)"
        $example_result
      }
    )

    # print $"This is the example data:"
    # print $"description_text: ($description_text)"
    # print $"example_text    : ($example_text)"
    # print $"example_result  : ($example_result)(ansi reset)"
    # print ""

    $example_count = $example_count + 1
    $out_text = $out_text + $"# This is the custom command ($example_count) for ($command_name):\n"
    if ($command_name | str contains 'ansi') {
      # ansi commands are just weird so they need special handling. Even with this they need
      # hand tweaking.
      let ansi_custom_command = (
        "#[test]\n" +
        $"def ($command_name)_($description_text)_($example_count) [] {\n" +
        "  let result = (" + $example_text + ")\n" +
        $"  assert (char lp)$result == (char dq)($example_result_output)(char dq)(char rp)\n" +
        "}"
      )
      $out_text = $out_text + $"\n($ansi_custom_command)\n"
    } else {
      let custom_command = (
        "#[test]\n" +
        $"def ($command_name)_($description_text)_($example_count) [] {\n" +
        "  let result = (" + $example_text + ")\n" +
        $"  assert (char lp)$result == ($example_result_output)(char rp)\n" +
        "}"
      )
      $out_text = $out_text + $"\n($custom_command)\n"
    }
    $out_text = $out_text + "\n"
    # print $out_text
  }

  $out_text
}