# set up git hooks to run:
# - `toolkit fmt --check --verbose` on `git commit`
# - `toolkit fmt --check --verbose` and `toolkit clippy --verbose` on `git push`
export def setup-git-hooks [] {
    print "This command will change your local git configuration and hence modify your development workflow. Are you sure you want to continue? [y]"
    if (input) == "y" {
        print $"running ('toolkit setup-git-hooks' | pretty-format-command)"
        git config --local core.hooksPath .githooks
    } else {
        print $"aborting ('toolkit setup-git-hooks' | pretty-format-command)"
    }
}
