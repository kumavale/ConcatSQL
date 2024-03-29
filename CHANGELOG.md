# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.1] - 2023-02-14
### Fixed
- Fix README examples

## [0.5.0] - 2023-02-14
### Added
- Add `query!` macro
- Add `concatsql_macro` crate

## [0.4.0] - 2021-01-20
### Added
- Add std::error::Error trait for Error
- Add prep function (#26)
- Add WrapString methods (#30)
- Supported arrays of different types (#31)
- Supported IpAddr and Time types (#32)

## [0.3.0] - 2020-12-04
### Added
- `impl Add<string arrays> for WrapString`
- Iterator and Index for `Row`
- UUID

### Changed
- Fix typo in document
- Improve performance

### Removed
- `check_valid_literal` function

### Fixed
- Fix memory leak in mysql and postgres

## [0.2.1] - 2020-11-25
### Fixed
- Fix error message memory leak
- Fix Row struct bug
- Fix bind bug for sqlite

## [0.2.0] - 2020-11-22
### Added
- Row::column method
- `impl Add<std::borrow::Cow<'_, str>> for WrapString`
- `impl Add<Option<T>> for WrapString`

### Changed
- Use static placeholders to query the database.
- Improve Row struct
- Rename to `WrapString::simulete()` from `WrapString::actual_sql()`

### Fixed
- Fix memory leak

## [0.1.1] - 2020-11-16
### Added
- Error type (ColumnNotFound)

### Changed
- Changed `without_escape` from method to independent function
- Changed the behavior of the `get_into` method

### Fixed
- Document

## [0.1.0] - 2020-11-16
- Initial release

