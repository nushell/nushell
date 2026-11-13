def main [name: string, count: int, --verbose, --tag: string = "dev"] {
    print $"($name):($count)"
    print ($env.PATH | describe)
}
