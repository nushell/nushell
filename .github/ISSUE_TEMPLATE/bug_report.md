---
name: Bug report
about: Create a report to help us improve
title: ''
labels: ''
assignees: ''

---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1.
2.
3.

**Expected behavior**
A clear and concise description of what you expected to happen.

**Screenshots**
If applicable, add screenshots to help explain your problem.

**Configuration (please complete the following information):**

Run `version | pivot` and paste the output to show OS, features, etc.

```
> version | pivot
╭───┬────────────────────┬───────────────────────────────────────────────────────────────────────╮
│ # │ Column0            │ Column1                                                               │
├───┼────────────────────┼───────────────────────────────────────────────────────────────────────┤
│ 0 │ version            │ 0.24.1                                                                │
│ 1 │ build_os           │ macos-x86_64                                                          │
│ 2 │ rust_version       │ rustc 1.48.0                                                          │
│ 3 │ cargo_version      │ cargo 1.48.0                                                          │
│ 4 │ pkg_version        │ 0.24.1                                                                │
│ 5 │ build_time         │ 2020-12-18 09:54:09                                                   │
│ 6 │ build_rust_channel │ release                                                               │
│ 7 │ features           │ ctrlc, default, directories, dirs, git, ichwh, ptree, rich-benchmark, │
│   │                    │ rustyline, term, uuid, which, zip                                     │
╰───┴────────────────────┴───────────────────────────────────────────────────────────────────────╯
```


**Add any other context about the problem here.**
