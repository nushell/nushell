#!/usr/bin/env python3
"""It does the same as the python example plugin"""
from nushell_sdk import NuPlugin


class GeneratorPlugin(NuPlugin):
    "Signature test for Python"
    name = "py-generate"

    def signature(self):
        return {
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
            "search_terms": ["Python", "Example"],
        }

    def call(self, input, *args, **kwargs):
        return [{"one": 0, "two": index, "three": 2 * index} for index in range(10)]


if __name__ == "__main__":
    GeneratorPlugin().run()
