# Release Note Generation

In order to more easily generate our release notes, we have a "Release notes summary" field in our PR template. This section is extracted from each PR merged for a release, and based on the labels applied to the PR the section is added to the appropriate section of the release notes. After the release notes have been compiled from the pull requests, we do an editing pass to add highlights, reorder sections, and tweak things as needed.

## Labels


| Label         | Meaning |
| ------------- | -------- |
| `notes:ready` | The "Release notes summary" section of the PR is ready to be included in the release notes |
| `notes:breaking-changes` | Includes summary in the "Breaking Changes" section of the release notes. |
| `notes:additions` | Includes summary in the "Additions" section of the release notes. |
| `notes:deprecations` | Includes summary in the "Deprecations" section of the release notes. |
| `notes:removals` | Includes summary in the "Removals" section of the release notes. |
| `notes:other` | Includes summary in the "Other changes" section of the release notes.|
| `notes:fixes` | Includes summary in the "Bug fixes" section of the release notes. |
| `notes:mention` | Includes summary in the "Hall of Fame" section of the release notes. |


## Labeling and generation rules
- When the release notes summary section of a PR is complete, the `notes:ready` label should be applied.
- Each PR should have *exactly one* category label applied. If no category label is applied, then it will default to the "Hall of Fame" section.
    - Any PRs which have defaulted to the "Hall of Fame" section will be called out by the script.
    - The "Hall of Fame" section cannot have multi-line summaries.
- PRs without a release notes summary are added to the "Hall of Fame" section.
    - This can be explicitly indicated by writing "N/A" in the release notes summary section
    - If the release notes summary section is empty, the `notes:ready` label can be used to indicate this was intentional. Otherwise, the script will print a notice but still add it to the "Hall of Fame" section.
- PR summaries will be sorted in order of length within each section.
- Multi-line summaries may have a third-level heading in the PR description. This will be used as the third-level heading in the release notes.
     - If no third-level heading is provided, the PR title will be used instead (cleaned up a bit, for example `feat: foo` turns into just `foo`). The script will print a notice when this happens
- Single line summaries will be generated in a bullet pointed list within its own heading.
     - This should always be the last heading within any section.
     - The heading can be omitted if all PR summaries in a section are a single line.

## Example

Here's a couple example PR release note summary sections, and the release notes generated that would be generated. For this example, all of these PRs are tagged with `notes:ready` and `notes:fixes`.

### PR 1
```md
## Release notes summary

Fixed a minor issue. This issue was caused by XYZ.
```

### PR 2
```md
## Release notes summary

### Fixed a complicated issue

This issue is complicated.

This affects code that is like XYZ.
```

### PR 3
````md
## Release notes summary

### Fixed foo bar issue

Fixed a difficult to explain issue. This is complicated.

```nu
example code that demonstrates the bug
```
````

### PR 4

```md
## Release notes summary

Fixed another minor issue.
```


### Generated release notes

````md
## Bug fixes

### Fixed foo bar issue
Fixed a difficult to explain issue. This is complicated.

```nu
example code that demonstrates the bug
```

### Fixed a complicated issue

This issue is complicated.

This affects code that is like XYZ.

### Additional bug fixes
- Fixed a minor issue. This issue was caused by XYZ.
- Fixed another minor issue.
````
