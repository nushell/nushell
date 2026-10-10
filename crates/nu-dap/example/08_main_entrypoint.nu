# Main entrypoint: `def main` makes this a proper CLI script.
# The debugger will prompt for arguments when launching.
# Try: name="world" count=3 --verbose --tag="release"

def main [
    name: string        # Required: the name to greet
    count: int          # Required: how many times to repeat
    --verbose           # Optional flag: show extra info
    --tag: string = "dev"  # Optional flag with default
] {
    if $verbose {
        print $"[DEBUG] name=($name) count=($count) tag=($tag)"
        print $"[DEBUG] working dir: ($env.PWD)"
    }

    for i in 1..$count {
        let msg = $"[($tag)] ($i)/($count): Hello, ($name)!"
        print $msg
    }

    print "Done."
}
