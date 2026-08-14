# Validation

## Static Checks

Run:

```text
hoi4skill validate "<mod-root>"
```

Fix errors before reporting completion. Warnings can be reported when they need game-side verification.

When the user can supply a real HOI4 install or dependency mod roots, run:

```text
hoi4skill validate "<mod-root>" --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod"
```

With an indexed game/mod codebase, invented references are fatal errors instead of soft warnings. That includes unknown focus IDs, country tags, focus/idea/decision/event sprites, technologies, equipment types, wargoal types, ideologies, resources, building types, and indexed modifiers.

## Manual Syntax Checklist

- Braces are balanced in `.txt`, `.mod`, `.gfx`, and `.gui` files.
- Each event file has one or more top-level `add_namespace = ...` lines before event bodies.
- Event IDs use a declared namespace and a number in `1..200000`.
- Localisation files start with `l_simp_chinese:` or the correct language header.
- Localisation keys referenced by focuses, ideas, events, and decisions exist.
- New files use unique IDs and do not silently replace vanilla content.
- National-focus mutual exclusion uses exactly `mutually_exclusive = { focus = <id> }`; approximate spellings such as `mutual_exclusion` are fatal errors.
- Critical national-focus fields must match exact HOI4 names. Near matches such as `prerequisites`, `completion_rewards`, `relative_position`, `ai_willdo`, or `cancel_if_invald` are fatal errors.
- Generated national focuses must keep the full house template fields: `icon`, `x`, `y`, `cost`, `ai_will_do`, `available`, `bypass`, `cancel_if_invalid`, `continue_if_invalid`, `available_if_capitulated`, and `completion_reward`. Missing fields are fatal validation errors.
- Event structure also requires exact names: `add_namespace`, `is_triggered_only`, `fire_only_once`, and `mean_time_to_happen`. `namespace =` and near-match spellings are fatal errors.
- Effects use valid scopes for the target system.
- Effect and trigger names are checked against the local game documentation or Wiki index when not already proven by nearby mod code.

## In-Game Test Checklist

Use this when the user asks for launch-ready or Workshop-ready output:

1. Enable only this mod and required dependencies.
2. Start the game with `-debug`.
3. Check `Documents/Paradox Interactive/Hearts of Iron IV/logs/error.log`.
4. Start as the target country.
5. Confirm the feature appears in the expected UI.
6. Trigger the feature naturally or through console commands.
7. Confirm localisation, icons, effects, and event popups.

## Common Failure Modes

- Missing localisation UTF-8 BOM is a fatal validation error because HOI4 may fail to load localisation without it.
- Copied vanilla files can conflict with other mods and future patches.
- `add_building_construction` must run in a state scope.
- Focus rewards must be in valid country/state scopes.
- Misspelled focus fields such as `mutual_exclusion` are ignored by HOI4 and must fail static validation.
- Do not fix only known typo examples. The validator rejects near-match spellings for all critical national-focus fields.
- Event IDs can collide when namespaces or numbers are reused.
- `replace_path` can disable vanilla or other mod content unexpectedly.

## `replace_path`-aware scanning

Layered scanners treat roots in HOI4 load order: game, dependency Mods, then the
edited Mod. A higher layer's repeated `replace_path` declarations mask the same
relative subtree in every lower layer. Normal runs prune a masked directory
before recursion, so its files are neither opened nor added to the effective
symbol index.

For game-update adaptation only, pass `--replace-path-diagnostics`. This opt-in
mode reads masked files and records bounded path, size, timestamp, and content
hash evidence in `layered_scan`; it never adds masked symbols back to validation
or generation. Use `--max-replaced-files <n>` to cap detailed file rows (default
200). Leave diagnostic mode off for ordinary authoring and validation.
