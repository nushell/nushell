#!/usr/bin/env python
# Example of using a Python script as a Nushell plugin
#
# The example uses JSON encoding but it should be a similar process using
# msgpack to move data between Nushell and the plugin. The only difference
# would be that you need to use msgpack relative lib(like msgpack) to
# decode and encode information that is read and written to stdin and stdout
#
# To register the plugin use:
# 	plugin add <path-to-py-file>
#
# Be careful with the spans. Miette will crash if a span is outside the
# size of the contents vector. We strongly suggest using the span found in the
# plugin call head as in this example.
#
# The plugin will be run using the active Python implementation. If you are in
# a Python environment, that is the Python version that is used
#
# Note: To keep the plugin simple and without dependencies, the dictionaries that
#   represent the data transferred between Nushell and the plugin are kept as
#   native Python dictionaries. The encoding and decoding process could be improved
#   by using libraries like pydantic and marshmallow
#
# This plugin uses python3
# Note: To debug plugins write to stderr using sys.stderr.write
import sys
import json


NUSHELL_VERSION = "0.106.2"
PLUGIN_VERSION = "0.1.1"  # bump if you change commands!


def signatures():
    """
    Multiple signatures can be sent to Nushell. Each signature will be registered
    as a different plugin function in Nushell.

    In your plugin logic you can use the name of the signature to indicate what
    operation should be done with the plugin
    """
    return {
        "Signature": [
            {
                "sig": {
                    "name": "nu-python",
                    "description": "Signature test for Python",
                    "extra_description": "",
                    "required_positional": [
                        {
                            "name": "a",
                            "desc": "required integer value",
                            "shape": "Int",
                        },
                        {
                            "name": "b",
                            "desc": "required string value",
                            "shape": "String",
                        },
                    ],
                    "optional_positional": [
                        {
                            "name": "opt",
                            "desc": "Optional number",
                            "shape": "Int",
                        }
                    ],
                    "rest_positional": {
                        "name": "rest",
                        "desc": "rest value string",
                        "shape": "String",
                    },
                    "named": [
                        {
                            "long": "help",
                            "short": "h",
                            "arg": None,
                            "required": False,
                            "desc": "Display the help message for this command",
                        },
                        {
                            "long": "flag",
                            "short": "f",
                            "arg": None,
                            "required": False,
                            "desc": "a flag for the signature",
                        },
                        {
                            "long": "named",
                            "short": "n",
                            "arg": "String",
                            "required": False,
                            "desc": "named string",
                        },
                    ],
                    "input_output_types": [["Any", "Any"]],
                    "allow_variants_without_examples": True,
                    "search_terms": ["Python", "Example"],
                    "is_filter": False,
                    "creates_scope": False,
                    "allows_unknown_args": False,
                    "category": "Experimental",
                },
                "examples": [],
            }
        ]
    }


def process_call(id, plugin_call):
    """
    plugin_call is a dictionary with the information from the call
    It should contain:
            - The name of the call
            - The call data which includes the positional and named values
            - The input from the pipeline

    Use this information to implement your plugin logic
    """
    # Pretty printing the call to stderr
    sys.stderr.write(json.dumps(plugin_call, indent=4))
    sys.stderr.write("\n")

    # Get the span from the call
    span = plugin_call["call"]["head"]

    # Creates a Value of type List that will be encoded and sent to Nushell
    def f(x, y):
        return {"Int": {"val": x * y, "span": span}}

    value = {
        "Value": [
            {
                "List": {
                    "vals": [
                        {
                            "Record": {
                                "val": {
                                    "one": f(x, 0),
                                    "two": f(x, 1),
                                    "three": f(x, 2),
                                },
                                "span": span,
                            }
                        }
                        for x in range(0, 10)
                    ],
                    "span": span,
                }
            },
            None,
        ]
    }

    write_response(id, {"PipelineData": value})


def tell_nushell_encoding():
    sys.stdout.write(chr(4))
    for ch in "json":
        sys.stdout.write(chr(ord(ch)))
    sys.stdout.flush()


def tell_nushell_hello():
    """
    A `Hello` message is required at startup to inform nushell of the protocol capabilities and
    compatibility of the plugin. The version specified should be the version of nushell that this
    plugin was tested and developed against.
    """
    hello = {
        "Hello": {
            "protocol": "nu-plugin",  # always this value
            "version": NUSHELL_VERSION,
            "features": [],
        }
    }
    sys.stdout.write(json.dumps(hello))
    sys.stdout.write("\n")
    sys.stdout.flush()


def write_response(id, response):
    """
    Use this format to send a response to a plugin call. The ID of the plugin call is required.
    """
    wrapped_response = {
        "CallResponse": [
            id,
            response,
        ]
    }
    sys.stdout.write(json.dumps(wrapped_response))
    sys.stdout.write("\n")
    sys.stdout.flush()


def write_error(id, text, span=None):
    """
    Use this error format to send errors to nushell in response to a plugin call. The ID of the
    plugin call is required.
    """
    error = (
        {
            "Error": {
                "msg": "ERROR from plugin",
                "labels": [
                    {
                        "text": text,
                        "span": span,
                    }
                ],
            }
        }
        if span is not None
        else {
            "Error": {
                "msg": "ERROR from plugin",
                "help": text,
            }
        }
    )
    write_response(id, error)


def handle_input(input):
    if "Hello" in input:
        if input["Hello"]["version"] != NUSHELL_VERSION:
            exit(1)
        else:
            return
    elif input == "Goodbye":
        exit(0)
    elif "Call" in input:
        [id, plugin_call] = input["Call"]
        if plugin_call == "Metadata":
            write_response(
                id,
                {
                    "Metadata": {
                        "version": PLUGIN_VERSION,
                    }
                },
            )
        elif plugin_call == "Signature":
            write_response(id, signatures())
        elif "Run" in plugin_call:
            process_call(id, plugin_call["Run"])
        else:
            write_error(id, "Operation not supported: " + str(plugin_call))
    else:
        sys.stderr.write("Unknown message: " + str(input) + "\n")
        exit(1)


def plugin():
    tell_nushell_encoding()
    tell_nushell_hello()
    for line in sys.stdin:
        input = json.loads(line)
        handle_input(input)


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "--stdio":
        plugin()
    else:
        print("Run me from inside nushell!")