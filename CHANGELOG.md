# Changelog

All notable changes to Unity Crop are documented in this file.

## [Unreleased]

## [3.0.0] - 2026-08-25

### Added

- command-line arguments for input, output, recursion, thread count, and logging;
- recursive project scanning with Unity-specific default exclusions;
- multithreaded sprite-sheet processing;
- repeatable include and exclude glob patterns;
- path and filename templates with documented placeholders;
- collision policies: rename, skip, overwrite, and error;
- dry-run and fail-fast modes;
- coordinate validation, filename sanitization, and structured summaries;
- English and Russian documentation;
- MIT license, automated tests, CI, and release workflows.

### Changed

- decode each source PNG once instead of once per sprite;
- continue processing other inputs after a malformed sheet unless `--fail-fast` is used;
- preserve relative project directories in the default output layout;
- rename the produced executable to `unity-crop`.

[Unreleased]: https://github.com/Senya01/UnityCrop/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/Senya01/UnityCrop/compare/2.0.0...v3.0.0
