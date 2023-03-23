import abc
import datetime
import json
import re
import sys
import traceback
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import timedelta
from typing import Any


@dataclass
class FileSize:
    bytes: int

    def __mul__(self, other):
        return FileSize(self.bytes * other)

    def __add__(self, other):
        return FileSize(self.bytes + other)

    def __div__(self, other):
        return FileSize(self.bytes / other)

    def __sub__(self, other):
        return FileSize(self.bytes - other)

    @property
    def kilobytes(self):
        return self.bytes / 1000

    @property
    def megabytes(self):
        return self.kilobytes / 1000

    @property
    def gigabytes(self):
        return self.megabytes / 1000

    @property
    def terabytes(self):
        return self.gigabytes / 1000

    @property
    def petabytes(self):
        return self.terabytes / 1000

    @property
    def exabytes(self):
        return self.petabytes / 1000

    @property
    def zetatabytes(self):
        return self.exabytes / 1000

    @property
    def kibibytes(self):
        return self.bytes / 1024

    @property
    def mebibytes(self):
        return self.kibibytes / 1024

    @property
    def gibibytes(self):
        return self.mebibytes / 1024

    @property
    def tebibytes(self):
        return self.gibibytes / 1024

    @property
    def pebibytes(self):
        return self.tebibytes / 1024

    @property
    def exbibytes(self):
        return self.pebibytes / 1024

    @property
    def zebibytes(self):
        return self.exbibyte / 1024


@dataclass
class Range:
    start: int
    end: int
    increment: int
    is_inclusive: bool


@dataclass
class NushellObject:
    key: str
    value: Any


class NuPlugin(abc.ABC):
    _head_span = {"start": 0, "end": 1}

    @abc.abstractproperty
    def name(self) -> str:
        pass

    def signature(self):
        return {}

    def examples(self):
        return []

    @abc.abstractmethod
    def call(self, input):
        pass

    def decode(self, obj):
        key = list(obj.keys())[0]
        value = obj[key]
        if key == "Nothing":
            return None
        if key in ("Int", "Float", "Bool", "String"):
            return value["val"]
        if key == "Filesize":
            return FileSize(value["val"])
        if key == "Date":
            return datetime.datetime.fromisoformat(
                re.sub(r"\.(\d{6})\d*", r".\1", value["val"])
            )
        if key == "List":
            return [self.decode(item) for item in value["vals"]]
        if key == "Record":
            return {
                value["cols"][index]: self.decode(value["vals"][index])
                for index in range(len(value["cols"]))
            }
        if key == "Duration":
            return timedelta(microseconds=value["val"] / 1000)
        if key == "Range":
            return Range(
                start=self.decode(value["val"]["from"]),
                increment=self.decode(value["val"]["incr"]),
                end=self.decode(value["val"]["to"]),
                is_inclusive=(value["val"]["inclusion"] == "Inclusive"),
            )
        if key == "Binary":
            return bytes(value["val"])

        return self.decode_custom(key, value)

    def decode_custom(self, key, value):
        return NushellObject(key, value)

    def encode(self, obj):
        if obj is None:
            key = "Nothing"
            value = {}
        elif isinstance(obj, bool):
            key = "Bool"
            value = {"val": obj}
        elif isinstance(obj, FileSize):
            key = "Filesize"
            value = {"val": obj.bytes}
        elif isinstance(obj, timedelta):
            key = "Duration"
            value = {"val": int(obj.total_seconds() * 1_000_000_000)}
        elif isinstance(obj, int):
            key = "Int"
            value = {"val": obj}
        elif isinstance(obj, float):
            key = "Float"
            value = {"val": obj}
        elif isinstance(obj, str):
            key = "String"
            value = {"val": obj}
        elif isinstance(obj, datetime.datetime):
            key = "Date"
            value = {"val": obj.isoformat()}
        elif isinstance(obj, bytes):
            key = "Binary"
            value = {"val": list(obj)}
        elif isinstance(obj, Mapping):
            key = "Record"
            value = {
                "cols": list(obj.keys()),
                "vals": [self.encode(obj[key]) for key in obj.keys()],
            }
        elif isinstance(obj, Sequence):
            key = "List"
            value = {"vals": [self.encode(o) for o in obj]}
        elif isinstance(obj, Range):
            key = "Range"
            value = {
                "val": {
                    "from": self.encode(obj.start),
                    "incr": self.encode(obj.increment),
                    "to": self.encode(obj.end),
                    "inclusion": "Inclusive" if obj.is_inclusive else "RightExclusive",
                }
            }
        elif isinstance(obj, NushellObject):
            key = obj.key
            value = obj.value
        else:
            key, value = self.encode_custom(obj)
        value["span"] = self._head_span
        return {key: value}

    def encode_custom(self, obj):
        raise RuntimeError(f"Unhandled Python type: {type(obj)}")

    @staticmethod
    def __tell_nushell_encoding():
        sys.stdout.write(chr(4) + "json")
        sys.stdout.flush()

    def _build_signature(self):
        signature = {
            "name": self.name,
            "usage": self.__doc__,
            "extra_usage": "",
            "input_type": "Any",
            "output_type": "Any",
            "required_positional": [],
            "optional_positional": [],
            "vectorizes_over_list": False,
            "named": [],
            "input_output_types": [["Any", "Any"]],
            "allow_variants_without_examples": True,
            "search_terms": [],
            "is_filter": False,
            "creates_scope": False,
            "allows_unknown_args": False,
            "category": "Experimental",
        }
        signature.update(self.signature())
        return signature

    def _process_call(self, plugin_call):
        self._head_span = plugin_call["CallInfo"]["call"]["head"]
        args = [
            self.decode(arg) for arg in plugin_call["CallInfo"]["call"]["positional"]
        ]
        kwargs = {
            arg[0]["item"]: True if arg[1] is None else self.decode(arg[1])
            for arg in plugin_call["CallInfo"]["call"]["named"]
        }

        input_data = self.decode(plugin_call["CallInfo"]["input"]["Value"])
        output_data = self.call(input_data, *args, **kwargs)
        return {"Value": self.encode(output_data)}

    @staticmethod
    def print(*args):
        print(*args, file=sys.stderr)

    def error(self, message):
        return {
            "Error": {
                "label": "ERROR from Python plugin",
                "msg": message,
                "span": self._head_span,
            }
        }

    def run(self):
        self.__tell_nushell_encoding()
        call_str = ",".join(sys.stdin.readlines())
        plugin_call = json.loads(call_str)

        if plugin_call == "Signature":
            response = {
                "Signature": [
                    {"sig": self._build_signature(), "examples": self.examples()}
                ]
            }
        elif "CallInfo" in plugin_call:
            try:
                response = self._process_call(plugin_call)
            except Exception as ex:
                response = self.error(str(ex) + "\n" + traceback.format_exc())
        else:
            response = self.error("Unknown call from Nushell to Python")
        sys.stdout.write(json.dumps(response))
