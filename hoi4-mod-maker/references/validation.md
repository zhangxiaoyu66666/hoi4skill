# Validation

## Static Checks

Run:

```text
hoi4skill validate "<mod-root>"
```

Fix errors before reporting completion. Warnings can be reported when they need game-side verification.

## Manual Syntax Checklist

- Braces are balanced in `.txt`, `.mod`, `.gfx`, and `.gui` files.
- Each event file has one or more top-level `add_namespace = ...` lines before event bodies.
- Event IDs use a declared namespace and a number in `1..200000`.
- Localisation files start with `l_simp_chinese:` or the correct language header.
- Localisation keys referenced by focuses, ideas, events, and decisions exist.
- New files use unique IDs and do not silently replace vanilla content.
- National-focus mutual exclusion uses exactly `mutually_exclusive = { focus = <id> }`; approximate spellings such as `mutual_exclusion` are fatal errors.
- Critical national-focus fields must match exact HOI4 names. Near matches such as `prerequisites`, `completion_rewards`, `relative_position`, `ai_willdo`, or `cancel_if_invald` are fatal errors.
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
