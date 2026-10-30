# Changelog

All notable changes to `uma-stable-match-scoring` are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows the [versioning policy](README.md#versioning) in its README.

## [Unreleased]

### Added

- `blended_pace` and `blended_median`: early-month paces that blend this month's data with a prior.

## [0.1.0] - Unreleased

First release.

### Added

- `score_clubs` and `score_club`: rank clubs by how well they fit a trainer's daily fan pace, optionally weighted by club balance.
- `club_balance`: how evenly a club's current members contribute.
- `daily_pace` and `current_members` for preparing inputs.
- `tier_label` for club tier codes.
- `Club`, `MemberSnapshot` and `ScoringOptions` input types, with constructors.
- Optional `serde` feature for serialising and deserialising the public data types.
- Minimum supported Rust version: 1.71.
