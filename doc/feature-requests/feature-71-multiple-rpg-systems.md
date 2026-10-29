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
- The current system is static, with no computed fields or rule automation

### Audit (2026-10-29)

The data-driven half of this feature has since been built; the scripting/plugin
half has not.

- A JSON-based module system exists (`src/modules/mod.rs`, `ModuleKind::System`)
  with declarative roll rules (`RollRules`, around `src/modules/mod.rs:107`) and
  a loader (`src/modules/loader.rs`). RPG systems can be described as data.
- Missing: the scripting or plug-in layer the description calls for (JavaScript,
  AssemblyScript, a custom DSL, or Compact Pascal) to implement rules that pure
  static data cannot express, such as computed fields and rule automation. The
  options listed above have not been evaluated or chosen.
