# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Scoring Weight Rebalancing**: Adjusted scoring factor weights from 50%/35%/15% to 40%/40%/20% (Coverage/Complexity/Dependency)
  - Coverage factor reduced from 50% to 40% to balance its influence
  - Complexity factor increased from 35% to 40% to better reflect code maintainability concerns
  - Dependency factor increased from 15% to 20% to better capture architectural risk and impact
  - This rebalancing provides more nuanced prioritization by giving complexity and dependencies greater weight in the final score while still maintaining coverage as a critical factor
