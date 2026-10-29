# Feature 62: Firefox ESR Automated Testing

Automate testing on Firefox 115.32.0 ESR, ideally as an MCP server.

## Description

Devise a way to automate testing on Firefox 115.32.0 ESR. Ideally implemented as an MCP server so it can be invoked during development sessions.

Firefox ESR is a supported browser for this project, so compatibility must be verified.

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

### Audit (2026-10-29)

General browser-automation tooling for development is documented, but nothing
specific to Firefox ESR exists.

- `DEV.md:129-139` describes the dev browser-automation setup (geckodriver /
  chromedriver and the rust-browser-mcp server) for use during sessions.
- Missing: a pin to Firefox 115.32.0 ESR specifically, any ESR compatibility
  test, and any CI job that runs browser tests. The MCP-server goal in the
  description is partly met by rust-browser-mcp, but it is not wired to ESR.
