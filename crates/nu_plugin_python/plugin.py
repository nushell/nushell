# Example of using python as script to create plugins for nushell
#
# The example uses JSON encoding but it should be a similar process using
# capnp proto to move data betwee nushell and the plugin. The only difference
# would be that you need to compile the schema file in order have the objects
# that decode and encode information that is read and written to stdin and stdour
#
# To register the plugin use:
# 	register <path-to-py-file> -e json
#
# Be carefull with the spans. Miette will crash if a span is outside the
# size of the contents vector. For this example we are using 0 and 1, which will
# point to the beginning of the contents vector. We strongly suggest using the span
# found in the plugin call head
#
# The plugin will be run using the active python implementation. If you are in
# a python environment, that is the python version that is used
#
# Note: To keep the plugin simple and without dependencies, the dictionaries that
#   represent the data transferred between nushell and the plugin are kept as
#   native python dictionaries. The encoding and decoding process could be improved
#   by using libraries like pydantic and marshmallow
#
# This plugin uses python3
# Note: To debug plugins write to stderr using sys.stderr.write
import sys
import json


def signatures():
    """
    Multiple signatures can be sent to nushell. Each signature will be registered
    as a different plugin function in nushell.

    In your plugin logic you can use the name of the signature to indicate what
    operation should be done with the plugin
    """
    return {
        "Signature": [
            {
                "name": "nu-python",
                "usage": "Signature test for python",
                "extra_usage": "",
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
                        "desc": "Display this help message",
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
            - The input from the pippeline

    Use this information to implement your plugin logic
    """
    # Pretty printing the call to stderr
    sys.stderr.write(json.dumps(plugin_call, indent=4))
    sys.stderr.write("\n")

    # Creates a Value of type List that will be encoded and sent to nushell
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


def plugin():
    call_str = ",".join(sys.stdin.readlines())
    plugin_call = json.loads(call_str)

    if plugin_call == "Signature":
        signature = json.dumps(signatures())
        sys.stdout.write(signature)

    elif "CallInfo" in plugin_call:
        response = process_call(plugin_call)
        sys.stdout.write(json.dumps(response))

    else:
        # Use this error format if you want to return an error back to nushell
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
