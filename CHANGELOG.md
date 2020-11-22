# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2020-11-22
### Added
    - Row::column method
    - impl Add<std::borrow::Cow<'_, str>> for WrapString
    - impl Add<Option<T>> for WrapString

### Changed
    - Use static placeholders to query the database.
    - Improve Row struct
    - Rename to WrapString::simulete() from WrapString::actual_sql()

### Fixed
    - Fix memory leak

## [0.1.1] - 2020-11-16
### Added
    - Error type (ColumnNotFound)

### Changed
    - Changed without_escape from method to independent function
    - Changed the behavior of the get_into method

### Fixed
    - Document

## [0.1.0] - 2020-11-16
- Initial release

