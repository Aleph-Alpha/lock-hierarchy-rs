# Changelog

## [0.2.0](https://github.com/Aleph-Alpha/lock-hierarchy-rs/compare/v0.1.3...v0.2.0) - 2025-02-12

### Added

- Remove escape hatch caused by returning the inner RwLock Guard on poisoned
- Debug and Display for all Guards
- Add RwLock
- Add Mutex::get_mut

### Other

- Multithreaded tests
- Unify tests
- Mutex to separate module
- Lock level mechanism to new struct
- Run clippy also on the release profile
- Gitignore

## [0.1.3](https://github.com/Aleph-Alpha/lock-hierarchy-rs/compare/v0.1.2...v0.1.3) - 2025-02-07

### Added

- implement Debug, Default and From for Mutex

### Fixed

- fix linebreaks in assertion
- fix doctest

### Other

- avoid misaligned comments by rustfmt
- use const initializer as suggested by clippy
- add lint workflow that runs clippy and rustfmt
- rename release-plz -> release-plz.yml
- use LF (Unix style line endings) instead of CRLF everywhere
- Ellide lifetimes
- Introduce release-plz
- Rename Chanelog to uppercase
- additional test for lock hierarchie violations
- improve error message on lock hierarchie violation
- extract integration tests
- LOCK_LEVELS is private
- add comments

## 0.1.2

* Only perform assertions for debug builds.

## 0.1.1

First released version
