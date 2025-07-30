#!/usr/bin/env -S nu --stdin
# Example of using a Nushell script as a Nushell plugin
#
# This is a port of the nu_plugin_python_example plugin to Nushell itself. There is probably not
# really any reason to write a Nushell plugin in Nushell, but this is a fun proof of concept, and
# it also allows us to test the plugin interface with something manually implemented in a scripting
# language without adding any extra dependencies to our tests.

const NUSHELL_VERSION = "0.106.2"
const PLUGIN_VERSION = "0.1.1" # bump if you change commands!

def main [--stdio] {
  if ($stdio) {
    start_plugin
  } else {
    print -e "Run me from inside nushell!"
    exit 1
  }
}

const SIGNATURES = [
  {
    sig: {
      name: nu_plugin_nu_example,
      description: "Signature test for Nushell plugin in Nushell",
      extra_description: "",
      required_positional: [
        [
          name,
          desc,
          shape
        ];
        [
          a,
          "required integer value",
          Int
        ],
        [
          b,
          "required string value",
          String
        ]
      ],
      optional_positional: [
        [
          name,
          desc,
          shape
        ];
        [
          opt,
          "Optional number",
          Int
        ]
      ],
      rest_positional: {
        name: rest,
        desc: "rest value string",
        shape: String
      },
      named: [
        [
          long,
          short,
          arg,
          required,
          desc
        ];
        [
          help,
          h,
          null,
          false,
          "Display the help message for this command"
        ],
        [
          flag,
          f,
          null,
          false,
          "a flag for the signature"
        ],
        [
          named,
          n,
          String,
          false,
          "named string"
        ]
      ],
      input_output_types: [
        [Any, Any]
      ],
      allow_variants_without_examples: true,
      search_terms: [
        Example
      ],
      is_filter: false,
      creates_scope: false,
      allows_unknown_args: false,
      category: Experimental
    },
    examples: []
  }
]

def process_call [
  id: int,
  plugin_call: record<
    name: string,
    call: record<
      head: record<start: int, end: int>,
      positional: list,
      named: list,
    >,
    input: any
  >
] {
  # plugin_call is a dictionary with the information from the call
  # It should contain:
  #         - The name of the call
  #         - The call data which includes the positional and named values
  #         - The input from the pipeline

  # Use this information to implement your plugin logic

  # Print the call to stderr, in raw nuon and as a table
  $plugin_call | to nuon | print -e
  $plugin_call | table -e | print -e

  # Get the span from the call
  let span = $plugin_call.call.head

  # Create a Value of type List that will be encoded and sent to Nushell
  let value = {
    Value: [{
      List: {
        vals: (0..9 | each { |x|
          {
            Record: {
              val: (
                [one two three] |
                  zip (0..2 | each { |y|
                    {
                      Int: {
                        val: ($x * $y),
                        span: $span,
                      }
                    }
                  }) |
                  into record
              ),
              span: $span
            }
          }
        }),
        span: $span
      }
    }, null]
  }

  write_response $id { PipelineData: $value }
}

def tell_nushell_encoding [] {
  print -n "\u{0004}json"
}

def tell_nushell_hello [] {
  # A `Hello` message is required at startup to inform nushell of the protocol capabilities and
  # compatibility of the plugin. The version specified should be the version of nushell that this
  # plugin was tested and developed against.
  let hello = {
    Hello: {
      protocol: "nu-plugin", # always this value
      version: $NUSHELL_VERSION,
      features: []
    }
  }
  $hello | to json --raw | print
}

def write_response [id: int, response: record] {
  # Use this format to send a response to a plugin call. The ID of the plugin call is required.
  let wrapped_response = {
    CallResponse: [
      $id,
      $response,
    ]
  }
  $wrapped_response | to json --raw | print
}

def write_error [id: int, text: string, span?: record<start: int, end: int>] {
  # Use this error format to send errors to nushell in response to a plugin call. The ID of the
  # plugin call is required.
  let error = if ($span | is-not-empty) {
    {
      Error: {
        msg: "ERROR from plugin",
        labels: [
          {
            text: $text,
            span: $span,
          }
        ],
      }
    }
  } else {
    {
      Error: {
        msg: "ERROR from plugin",
        help: $text,
      }
    }
  }
  write_response $id $error
}

def handle_input []: any -> nothing {
  match $in {
    { Hello: $hello } => {
      if ($hello.version != $NUSHELL_VERSION) {
        exit 1
      }
    }
    "Goodbye" => {
      exit 0
    }
    { Call: [$id, $plugin_call] } => {
      match $plugin_call {
        "Metadata" => {
          write_response $id {
            Metadata: {
              version: $PLUGIN_VERSION
            }
          }
        }
        "Signature" => {
          write_response $id { Signature: $SIGNATURES }
        }
        { Run: $call_info } => {
          process_call $id $call_info
        }
        _ => {
          write_error $id $"Operation not supported: ($plugin_call | to json --raw)"
        }
      }
    }
    $other => {
      print -e $"Unknown message: ($other | to json --raw)"
      exit 1
    }
  }
}

def start_plugin [] {
  lines |
    prepend (do {
      # This is a hack so that we do this first, but we can also take input as a stream
      tell_nushell_encoding
      tell_nushell_hello
      []
    }) |
    each { from json | handle_input } |
    ignore
}
