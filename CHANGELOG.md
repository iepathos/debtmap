# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING**: God object detection now differentiates between god classes and god files (spec 130)
  - God Class detection: Only counts production methods within structs/classes, excluding test functions
  - God File detection: Counts all standalone functions in a file, including both production and test functions
  - Detection type is now properly reported in output to distinguish between GodClass and GodFile
  - This change may affect existing metrics if your codebase had god objects with significant test code
  - Method counts for god classes will be lower (more accurate) as test methods are now excluded
