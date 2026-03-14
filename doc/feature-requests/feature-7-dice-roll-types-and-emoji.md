# Feature 7: Dice Roll Enhancement - Expressions, Types and Emoji

## Expressions

Add arithmetic expressions to dice rolling syntax.

Currently 2d6+3 is possible. But we would like 1d20+1d4 and 2d6/2 or d10*10+d10

## Tagging Effect Types

Players can optionally tag dice rolls with a damage or effect type. Multiple
types can be specified with a comma. Different dice in an expression can be
tagged differently. The total for the roll is still displayed, but if there is
more than one type in the expression the individual totals each effect type are
also displayed.

For example: "3d8 (healing)" or "1d6 (slashing), 1d4 (fire)", "1d20+7 (to-hit)".

## Emoji

Add a dice emoji 🎲to roll messages.
Add 🔥to "fire" damage.
Add 🎯 to "to-hit" rolls.
Add ⚔️ to "physical" rolls. Add 🏹 to "ranged" rolls.
Add 🍰to "cake" rolls.
Add 🌊to "water" rolls.
Add ❄️to "cold" rolls. Add 🪨to "earth" rolls.
Add 🌪️to "wind" rolls.
Add ⚡to "electric" rolls.
Add 🌟to "magic" rolls.
Add ⛪to "holy" rolls.
Add 🌿 to "nature" rolls.
Add 💀to "death" and "death save" rolls.
Add 🧛 or 🧛‍♀️ or 🧟 or 🧟‍♂️to "undead" rolls.
Add 🧠to "mind" or "psychic" rolls.
Add 😢to rolls with a natural 1 (unmodified dice value is 1).
Add 😎 to d20 rolls that are a natural 20 (unmodified dice value is 20).

## Status: Not Started

Basic dice rolling (NdN+M) works but none of the three parts of this feature
are implemented: no compound arithmetic expressions (e.g. 1d20+1d4), no
damage/effect type tagging, and no emoji support.

## Plan

TBD

## Findings

- Current parser `parse_and_roll()` in `ws_handler.rs` only handles NdN with a
  single +/- modifier — no support for multiple dice groups, multiplication, or
  division
- Chat component `is_dice_roll()` in `chat.rs` detects dice notation for auto-rolling
