# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Conflict between `auto` and `target_fan_speeds` options.

- Support for setting fan speed by using the index.

- Support for `/dev/port`, allowing to install the service without installing
  kernel modules (on systems where `/dev/port` can be accessed).

### Changed

- Refactoring code:
  - `Arc` have been replaced by `Rc`, since the code is single-threaded now.
  - `Mutex` and `RwLock` have been replaced by `RefCell` where appropriated.
  - Remove `Unpin` trait.

### Fixed

- Check the number of speeds provided to `set_target_fans_speeds`.

## [v0.2.0] - 2020-09-25

### Added

- Support of other sensors than the CPU one for temperature computation.


## [v0.1.0]

Initial version.

[v0.2.0]: https://github.com/musikid/fancy/compare/fancy-service-0.1.0...fancy-service-0.2.0
[v0.1.0]: https://github.com/musikid/fancy/compare/fancy-service-0.1.0