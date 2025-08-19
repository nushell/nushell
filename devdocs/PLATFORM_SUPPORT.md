# Nushell platform support policy

Nushell envisions to be a cross-platform shell, despite taking some strong design inspiration from UNIX and POSIX command names and style conventions we explicitly support Windows.

## cross-platform design
This commitment to a cross-platform Nushell forces us to make provisions so users on Windows can have the generally same pleasant experience: e.g. supporting paths with backslash as the directory separator, forces us to support string literals that accept those.

In general our design strives to have a consistent behavior across all platforms if defining the semantics is possible for Nushell.
In some cases where the platform requirements dominate we may choose to follow the platform specific defaults. (some nuances around the file system)
Only rarely do we want to accept commands/language features that only support a single platform, only to access common system behavior of this particular platform (e.g. `registry query` command for the windows registry, `exec` for Linux and MacOS)

## cross-platform builds and testing

The Nushell team runs **testing of Nushell for the following platforms** through our CI:

- macOS (latest version available through GitHub CI)
- Windows (10 and 11)
- Linux (our test runners use `ubuntu-22.04` to represent distributions with not the latest glibc versions.)

All PR level tests are performed on x86/AMD64 (at least at the time of writing the default macOS runner was not yet using arm64).

As an additional layer of validation we perform [nightly builds](https://github.com/nushell/nightly/releases).

Those target **additional build targets**:
- **aarch64 for all platforms**
- musl as an alternative to Glibc on linux
- riscv only for linux
- armv7 only for linux
- loongarch64 only for linux (with limitations [^1])

We will try to provide builds for all of them but a standard configuration for x86-64 or aarch64 will take priority for us should we face technical challenges in a release cycle.

[^1]: The build for loongarch64 currently lacks support for the Nushell internal error recovery, as it doesn't compile with `rustc -C panic=unwind`. It has to use `panic=abort` thus bugs raising panics will abort your shell. Our other platforms by default have a limited capability to recover from non-fatal panics to provide a stable login shell.

### Supported feature flags

We have features of Nushell behind flags that can be passed at compilation time.

The design focus of Nushell is primarily expressed by everything accessible without passing additional feature flag.
This provides a standard command set and receives the most attention.

## Passively supported platforms

These platforms are not actively managed through our CI so may encounter unintended regressions.
Furthermore certain features may not yet be available, even though we are willing to accept PRs trying to close that gap.


- OpenBSD
    - e.g. missing the `ps` command
- FreeBSD
    - e.g. missing the `ps` command
- Android via Termux

Help from the community to make sure they get tested and improved so they can become first class targets would be greatly appreciated!


## Providing builds and packaging

The Nushell team only provides a select few distribution sources and so far encourages community members to maintain the individual packages for particular package managers:

We provide:
- source code distribution via `crates.io` -> `cargo install nu`
- GitHub builds with each release: (following the build matrix of the nightly builds)
- the setup for `winget` packaging

### For package maintainers:

We aim to support the rust version that is two releases behind the most recent version of stable Rust so the build infrastructure of your packaging environment can already be proven out.
