# Feature 14: Email Validation

Validate user email addresses.

## Status: Not Started

Email field is collected in signup and stored in the database, but no format
validation or verification is performed.

## Plan

TBD

## Findings

- Email field stored in database (`users` table)
- Email collected in `SignupRequest` struct (`models.rs`) and signup form
  (`pages/login.rs`)
- No validation regex or email verification flow in `server/api.rs` signup logic
