# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

<!-- Use this as a template
## [X.Y.Z] - YYYY-MM-DD
### Added
- for new features.

### Changed
- for changes in existing functionality.

### Deprecated
- for soon-to-be removed features.

### Removed
- for now removed features.

### Fixed
- for any bug fixes.

### Security
- in case of vulnerabilities.
-->
 
# [0.76.0] - 2023-02-21

### Changed

- Disable auto-benchmark harness for this crate
 
# [0.75.0] - 2023-01-31

### Changed

- Removing unnecessary comments from the code
- Use variable names directly in the format strings 
- Apply more recent/nightly clippy lints
 
# [0.73.0] - 2022-12-20

### Changed

- Remove unused dev-dependencies
- The lazy_static crate has been replaced by once_cell
 
# [0.71.0] - 2022-11-29

### Fixed

- Fixed json parsing
 
# [0.71.0] - 2022-11-08

### Changed

- Run a round of clippy --fix to fix a ton of lints

# [0.67.0] - 2022-08-16

### Added

- Add repository info to all workspace crates

### Changed

- Replace the regex crate with the fancy-regex crate
 
# [0.66.0] - 2022-07-26

### Fixed

- Prevents panic when parsing JSON containing large number
 
# [0.64.0] - 2022-06-15

### Changed

- Address lints from clippy for beta/nightly

# [0.60.1] - 2022-03-27

### Changed

- Align all of the serde_json crates to the same version (`serde_json = "0.1.39"` --> `serde_json = "0.1"`)

# [0.60.0] - 2022-03-22

### Added

- Adds tab indentation option for JSON files. 

### Changed

- Changing the name of the parent project: 'The Nu Project' --> 'The Nushell Project'

# [0.59.1] - 2022-03-02

### Added

- Add indent flag to json (first draft) 

### Fixed

- Fix to json escape logic
- Clippy fixes 

### Changed

- Update this cargo crate to edition 2021
- Strip trailing whitespace in files

# [0.42.0] - 2021-12-28

### Fixed

- fix issue #559: to json -r serializes datetime without spaces

### Changed

- add in a raw flag in the command to json

# [0.41.0] - 2021-12-07

### Changed

- avoid unnecessary allocation (serialization)

# [0.37.0] - 2021-09-14

### Changed

- Add general refactorings

# [0.31.0] - 2021-05-11

### Fixed

- Clippy fixes for new Rust version

# [0.29.2] - 2021-04-06

### Fixed

- Fix typos and capitalization of "Unicode"

# [0.29.0] - 2021-03-30

### Fixed

- Fix warnings for Rust 1.51

# [0.28.0] - 2021-03-09

### Changed

- Preserve order when serializing/deserialize json by default.

# [0.27.0] - 2021-02-16

### Fixed

- Fix latest clippy warnings

# [0.25.2] - 2021-01-11

### Changed

- Update num-traits requirement from 0.1.32 to 0.2.14

# [0.25.0] - 2021-01-05

### Fixed

- Rust 1.49 Clippy Fixes

# [0.22.0] - 2020-11-22

### Changed

- Fork of serde-hjson
- The crate added to the 'Nu Project'
- Added Cargo.toml
- LICENSE file added
- Bump version to 0.22 according to the parent project