# Feature 10: Release Notes Automation

Automatically add release notes or a changelog when a release is tagged.

## Status: Done

GitHub Actions release workflow uses `generate_release_notes: true` to
auto-generate release notes on tagged releases.

## Plan

N/A — feature is complete.

## Findings

- Release workflow at `.github/workflows/release.yml` uses GitHub's built-in
  `generate_release_notes: true` in the release creation step
