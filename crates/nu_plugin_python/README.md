# Nushell Python plugin SDK

> **⚠ Warning: this SDK is not finalized yet.** While it tries to be bug-free, the architectural decisions may change in the future.

The main goal to provide a seamless integration between Nushell and Python.

## Quick start

Have `nu_plugin_py-quickstart.py` file with the following content:

```
from nushell_sdk import NuPlugin


class QuickStartPlugin(NuPlugin):
    """The simplest Python plugin for Nushell."""

    name = "py-quickstart"

    def call(self, input):
        if input == "magic":
            return 42
        else:
            return 0


if __name__ == "__main__":
    QuickStartPlugin().run()
```

Register it:

```
register nu_plugin_py-quickstart.py
```

The registered command is `py-quickstart`, the usage comes from the documentation comment from the class. When the command is executed, the `call` method is called. The second argument is always the input, which is a native Python type. The return of the method will be the output of the Nushell command.

```
❯ py-quickstart
0
❯ 123 | py-quickstart
0
❯ "magic" | py-quickstart
42
```

## Mapping between Nushell and Python types

|Nushell        |Python                |
|---------------|----------------------|
|Integer        |int                   |
|Float          |float                 |
|String         |str                   |
|Boolean        |bool                  |
|Date           |datetime.datetime     |
|Duration       |datetime.timedelta    |
|File size      |FileSize (custom)     |
|Range          |Range (custom)        |
|Binary         |bytes                 |
|List           |list                  |
|Record         |dict                  |
|Table          |list of dicts         |
|Null           |None                  |
|Everything else|NushellObject (custom)|

## Raise error

If an exception raises in Python, a Nushell error is made with the relevant information and stack trace.

## Parameters and signature

The parameters can be specified in signature, using the Nushell syntax for it. The signature is prefilled with basic default options, so you have to fill only the relevant parts. For more information about Nushell pulgin signature, see the [Plugins chapter of the Nushell book](https://www.nushell.sh/book/plugins.html).

If you want to provide examples, you can override the `examples` method.


## Limitations

While Nushell plugin architecture allows to set multiple signatures (with different command name), at current implementation only one signature can be provided for a Python plugin. If you want to provide more commands, you need to split them into different Python modules

## Custom encode / decode

If you have custom types in Nushell, you can override the `decode_custom` and `encode_custom` methods.

## Debug your code

If you need to debug your code, you can call `self.print`, which prints to `stderr`. (`stdout` is used to communicate between Python and Nushell.)

## Pure Python code

You can see a pure Python implementation (without this SDK) of a Nushell plugin in [nu_plugin_python_example.py](nu_plugin_python_example.py).