# Feature 66: Turn Start Notification

Visual feedback when a character's turn begins in initiative.

## Description

When a character's turn begins (via next turn or clicking in the initiative window):

- Briefly flash (2 times) the title bar of that character's sheet window
- Flash a twelve-pointed star outline around the character's token on the map — can rotate/spin, leveraging existing ping animation code
- If the client does not have that character sheet open, no window flash occurs

Add a status area at the bottom of the character window (like the File Viewer status bar). Display "Your Turn" in that status while the character's turn is active in initiative.

## Dependencies

- **Feature 52: Initiative Map Integration** — relies on initiative-to-token linkage

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
