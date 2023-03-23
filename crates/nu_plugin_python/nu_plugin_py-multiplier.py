#!/usr/bin/env python
import dataclasses
import datetime
from collections.abc import Mapping, Sequence

from nushell_sdk import FileSize, NuPlugin, Range


class MultiplierPlugin(NuPlugin):
    """Multiply the input data with the given amount."""

    name = "py-multiplier"

    def signature(self):
        return {
            "required_positional": [
                {
                    "name": "amount",
                    "desc": "Multiply input data by this amount",
                    "shape": "Int",
                    "var_id": None,
                },
            ],
            "named": [
                {
                    "long": "debug",
                    "short": "d",
                    "arg": None,
                    "required": False,
                    "desc": "Print debug information",
                    "var_id": None,
                },
            ],
        }

    def multiply(self, data, amount):
        if data == "Please make an error.":
            raise Exception("Here is an error for you.")
        if isinstance(data, bool):
            return data
        if (
            isinstance(data, datetime.timedelta)
            or isinstance(data, FileSize)
            or isinstance(data, int)
            or isinstance(data, float)
            or isinstance(data, str)
            or isinstance(data, bytes)
        ):
            return data * amount
        if isinstance(data, Range):
            return dataclasses.replace(
                data, end=data.start + (data.end - data.start) * amount
            )
        if isinstance(data, Mapping):
            return {key: self.multiply(value, amount) for key, value in data.items()}
        if isinstance(data, Sequence):
            return [self.multiply(value, amount) for value in data]
        return data

    def call(self, input, amount, debug=False):
        if debug:
            self.print("Input value:", input)
            self.print("Amount:", amount)
        multiplied = self.multiply(input, amount)
        if debug:
            self.print("Output value:", multiplied)
        return multiplied


if __name__ == "__main__":
    MultiplierPlugin().run()
