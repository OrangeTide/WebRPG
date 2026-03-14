# Unit Tests TODO

## High Priority

- [ ] `parse_and_roll` — `src/server/ws_handler.rs` — Dice notation parser (`2d6`, `1d20+5`, `d6-2`)
  - Valid notation: `2d6`, `1d20+5`, `d6-2`, `d20`, `100d6`
  - Edge cases: `0d6`, `1d0`, negative modifiers
  - Invalid input: `abc`, empty string, `dd6`
  - Result bounds: verify rolls fall within expected min/max

- [ ] `ability_modifier` — `src/server/ws_handler.rs` — D&D ability score → modifier
  - Standard: 10→0, 12→+1, 8→-1, 20→+5, 1→-5
  - Edge values: 0, very high scores

- [ ] `is_dice_roll` — `src/components/chat.rs` — Validate dice notation strings
  - Valid: `2d6`, `d20`, `1d6+3`, `1d6-2`
  - Invalid: `hello`, `2d`, `d`, empty string

## Medium Priority

- [ ] `hash_password` / `verify_password` — `src/auth.rs` — Argon2 password hashing
  - Round-trip: hash then verify same password succeeds
  - Wrong password fails verification

- [ ] `generate_jwt` / `verify_jwt` — `src/auth.rs` — JWT token handling
  - Round-trip token generation and validation
  - Expired token rejection
  - Requires `SECRET_KEY` env var in test setup
