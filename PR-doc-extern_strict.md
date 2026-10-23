# Add `extern --strict` to reject undeclared flags

## Description

An `extern` signature drives completions, but it doesn't guard anything. `extern "git status" [--short(-s)]; git status --shrt` passes `--shrt` to git, and git prints its own usage error. That's because externs accept unknown arguments, so nushell can wrap a tool without declaring every flag it has.

`extern --strict` (and `export extern --strict`) turns a flag the signature doesn't declare into a parse error, with the "Did you mean" hint that internal commands already give. Everything else an extern accepts still works: positional arguments, `--` and the arguments after it all reach the tool. `--help` is always allowed, so `git status --help` still shows git's help. `-h` isn't exempt, since it means something else for some tools (`ls -h`, `grep -h`).

`Signature` gets a `rejects_unknown_flags` field for this. `allows_unknown_args` stays true for externs, because it also decides whether `--` is forwarded and how completion and glob expansion treat the arguments.

### Before and after

Before is main at `f196eaf8a`.

| Command | Before | After |
| --- | --- | --- |
| `extern --strict "git status" [--short(-s)]; git status --shrt` | ``The `extern` command doesn't have flag `strict`.`` | ``The `git status` command doesn't have flag `shrt`.``, help: ``Did you mean: `--short`?`` |
| `extern --strict "git status" [--short(-s)]; git status -x` | same `strict` error | ``The `git status` command doesn't have flag `-x`.`` |
| `extern --strict "git status" [--short(-s)]; git status -s -- --shrt` | same `strict` error | runs `git status -s -- --shrt` |
| `extern --strict "git status" [--short(-s)]; git status --help` | same `strict` error | git's help |
| `module g { export extern --strict "git status" [--short(-s)] }; use g "git status"; git status --shrt` | same `strict` error | ``The `git status` command doesn't have flag `shrt`.`` |
| `extern "git status" [--short(-s)]; git status --shrt` | git: ``error: unknown option `shrt'`` | same |

```
Error: nu::parser::unknown_flag

  × The `git status` command doesn't have flag `shrt`.
   ╭─[source:1:56]
 1 │ extern --strict "git status" [--short(-s)]; git status --shrt
   ·                                                        ───┬──
   ·                                                           ╰── unknown flag
   ╰────
  help: Did you mean: `--short`?
```

## User-facing changes (Release notes)

- `extern --strict` makes a flag that the signature doesn't declare a parse error, with a "Did you mean" hint. `--`, the arguments after it, and `--help` still reach the external command.

## Additional notes

- `rejects_unknown_flags` has `#[serde(default)]`, so plugin signatures and plugin registry files from older builds still load.
- `export extern` inside a module read the extern's name from the word after `extern`, which is the flag in `export extern --strict name [...]`. It now skips flags, the same way the definition predeclaration already did.
