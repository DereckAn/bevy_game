---
alwaysApply: true
---

# Code Quality

## Anti-defaults (counter common Claude tendencies)

- No premature abstractions. Three similar lines beats a helper used once.
- Don't add features or improvements beyond what was asked.
- Don't refactor adjacent code while fixing a bug.
- No dead code or commented-out blocks. Git has history.
- WHY comments, never WHAT. If code needs a "what" comment, rename instead.
- API docs at module boundaries only, not every internal function.

## Naming

- Files and modules: `snake_case` (`chunk_loading.rs`, `greedy_meshing.rs`). One `mod.rs` per module dir.
- Types, traits, enums, enum variants: `PascalCase` (`BaseChunk`, `VoxelType`, `GameState::InGame`).
- Functions, methods, variables, fields: `snake_case` (`generate_chunk`). Constants/statics: `SCREAMING_SNAKE`.
- Booleans: `is_` / `has_` / `should_` prefix. Plugins end in `Plugin`, Bevy components/resources are nouns.
- Abbreviations only when universally known (`id`, `pos`, `idx`, `aabb`). Match existing acronym casing in the codebase.

## Code Markers

`TODO(author): desc (#issue)` for planned work. `FIXME(author): desc (#issue)` for known bugs. `HACK(author): desc (#issue)` for ugly workarounds (explain the proper fix). `NOTE: desc` for non-obvious context. Owner and issue link required. Never `XXX`, `TEMP`, `REMOVEME`.

## File Organization

- `use` order: `std`, external crates, then `crate::`/`super::`/`self::`. Blank line between groups.
- One plugin/system module per file. Register systems in the module's `Plugin` impl, not scattered.
- Function order: public API first, then helpers in call order.
