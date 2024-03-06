#!/usr/bin/env python
# Example of using a Python script as a Nushell plugin
#
# The example uses JSON encoding but it should be a similar process using
# msgpack to move data between Nushell and the plugin. The only difference
# would be that you need to use msgpack relative lib(like msgpack) to
# decode and encode information that is read and written to stdin and stdout
#
# To register the plugin use:
# 	register <path-to-py-file>
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
                    "usage": "Signature test for Python",
                    "extra_usage": "",
                    "input_type": "Any",
                    "output_type": "Any",
                    "required_positional": [
                        {
                            "name": "a",
                            "desc": "required integer value",
                            "shape": "Int",
                            "var_id": None,
                        },
                        {
                            "name": "b",
                            "desc": "required string value",
                            "shape": "String",
                            "var_id": None,
                        },
                    ],
                    "optional_positional": [
                        {
                            "name": "opt",
                            "desc": "Optional number",
                            "shape": "Int",
                            "var_id": None,
                        }
                    ],
                    "rest_positional": {
                        "name": "rest",
                        "desc": "rest value string",
                        "shape": "String",
                        "var_id": None,
                    },
                    "named": [
                        {
                            "long": "help",
                            "short": "h",
                            "arg": None,
                            "required": False,
                            "desc": "Display the help message for this command",
                            "var_id": None,
                        },
                        {
                            "long": "flag",
                            "short": "f",
                            "arg": None,
                            "required": False,
                            "desc": "a flag for the signature",
                            "var_id": None,
                        },
                        {
                            "long": "named",
                            "short": "n",
                            "arg": "String",
                            "required": False,
                            "desc": "named string",
                            "var_id": None,
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
    span = plugin_call["Run"]["call"]["head"]

    # Creates a Value of type List that will be encoded and sent to Nushell
    value = {
        "Value": {
            "List": {
                "vals": [
                    {
                        "Record": {
                            "val": {
                                "cols": ["one", "two", "three"],
                                "vals": [
                                    {
                                        "Int": {
                                            "val": x * y,
                                            "span": span
                                        }
                                    } for y in [0, 1, 2]
                                ]
                            },
                            "span": span
                        }
                    } for x in range(0, 10)
                ],
                "span": span
            }
        }
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
            "protocol": "nu-plugin", # always this value
            "version": "0.90.2",
            "features": []
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


def write_error(id, msg, span=None):
    """
    Use this error format to send errors to nushell in response to a plugin call. The ID of the
    plugin call is required.
    """
    error = {
        "Error": {
            "label": "ERROR from plugin",
            "msg": msg,
            "span": span
        }
    }
    write_response(id, error)


def handle_input(input):
    if "Hello" in input:
        return
    elif input == "Goodbye":
        return
    elif "Call" in input:
        [id, plugin_call] = input["Call"]
        if "Signature" in plugin_call:
            write_response(id, signatures())
        elif "Run" in plugin_call:
            process_call(id, plugin_call)
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
