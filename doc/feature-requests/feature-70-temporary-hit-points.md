# Feature 70: Temporary Hit Points

Add a temporary hit points field to the character sheet. Damage is applied to temp HP first. There is no max for temp HP — it is extra on top of max HP.

**HP bar rendering:**

- If temp HP = 0: bar width = max HP, filled with red for current HP, empty for the rest
- If temp HP > 0: bar width = max HP + temp HP, filled with red for current HP, blue for temp HP remainder
- When losing temp HP, the red portion grows (no empty gap between red and blue)
- Scale the bar to fit the graphical element regardless of total width

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

- Tokens have `current_hp` and `max_hp` fields
- HP bar rendering is in `src/components/map.rs` (around line 591)
- Character sheets have HP fields defined by the RPG template
- Need a new `temp_hp` field on tokens and/or characters
- Database migration needed for the new field
