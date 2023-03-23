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
