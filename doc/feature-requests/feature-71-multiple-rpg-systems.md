# Feature 71: Multiple RPG System Support

Support multiple RPG systems with definitions in a simple format (YAML or JSON). Each system may need scripting or a plug-in to implement fully, since pure static templates may not cover all rules.

**Options to evaluate:**

- JavaScript (runs in browser natively, wide ecosystem)
- AssemblyScript (TypeScript-like, compiles to WASM)
- Custom DSL (purpose-built for RPG rules, simple but limited)
- Compact Pascal (aligns with Feature 31, self-hosting compiler goal)

Need to research which approach balances flexibility, ease of authoring, and runtime safety.

## Dependencies

None (but related to Feature 31: Pascal Compiler).

## Status: Not Started

## Plan

(none yet)

## Findings

- Current template system is in `src/models.rs` (`TemplateInfo`, `TemplateField`)
- Templates define fields with types (Number, Text, Boolean) and categories
- Session creation picks a template; creatures and characters use it
- The current system is static — no computed fields or rule automation
