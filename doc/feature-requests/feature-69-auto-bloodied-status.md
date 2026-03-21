# Feature 69: Auto-Bloodied Status for Player Characters

Player character tokens should automatically update the "bloodied" condition based on HP. When HP is less than half max, the character is bloodied; when HP is more than half, the condition is removed.

**Important policy:** This automation applies only to player characters, not creatures. It is up to the GM to manage the bloodied status for creatures. Players should be roleplaying and asking the GM to describe the scene rather than relying on automation. Document this distinction in the help system.

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

- Tokens have a `conditions: Vec<String>` field with emoji-based condition icons
- HP updates come through `TokenHpUpdated` server messages
- The `condition_icon()` function in `map.rs` maps condition names to emoji
- Need to add "bloodied" to the condition icon mapping
- Auto-update logic should trigger on HP change for tokens linked to a character_id (not creature_id)
