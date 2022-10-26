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
# Be carefull with the spans. Miette will crash if a span is outside the
# size of the contents vector. For this example we are using 0 and 1, which will
# point to the beginning of the contents vector. We strongly suggest using the span
# found in the plugin call head
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
                        "var_id": None
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
                "search_terms": ["Python", "Example"],
                "is_filter": False,
                "creates_scope": False,
                "category": "Experimental",
            }
        ]
    }


def process_call(plugin_call):
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

    # Creates a Value of type List that will be encoded and sent to Nushell
    return {
        "Value": {
            "List": {
                "vals": [
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 1,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 2,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 2,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 4,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 3,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 6,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 4,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 8,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 5,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 10,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 6,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 12,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 7,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 14,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 8,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 16,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                    {
                        "Record": {
                            "cols": ["one", "two", "three"],
                            "vals": [
                                {
                                    "Int": {
                                        "val": 0,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 9,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                                {
                                    "Int": {
                                        "val": 18,
                                        "span": {"start": 0, "end": 1},
                                    }
                                },
                            ],
                            "span": {"start": 0, "end": 1},
                        }
                    },
                ],
                "span": {"start": 0, "end": 1},
            }
        }
    }


def tell_nushell_encoding():
    sys.stdout.write(chr(4))
    for ch in "json":
        sys.stdout.write(chr(ord(ch)))
    sys.stdout.flush()


def plugin():
    tell_nushell_encoding()
    call_str = ",".join(sys.stdin.readlines())
    plugin_call = json.loads(call_str)

    if plugin_call == "Signature":
        signature = json.dumps(signatures())
        sys.stdout.write(signature)

    elif "CallInfo" in plugin_call:
        response = process_call(plugin_call)
        sys.stdout.write(json.dumps(response))

    else:
        # Use this error format if you want to return an error back to Nushell
        error = {
            "Error": {
                "label": "ERROR from plugin",
                "msg": "error message pointing to call head span",
                "span": {"start": 0, "end": 1},
            }
        }
        sys.stdout.write(json.dumps(error))


if __name__ == "__main__":
    plugin()
