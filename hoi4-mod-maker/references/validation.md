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

- Missing localisation BOM may break older HOI4 localisation loading.
- Copied vanilla files can conflict with other mods and future patches.
- `add_building_construction` must run in a state scope.
- Focus rewards must be in valid country/state scopes.
- Event IDs can collide when namespaces or numbers are reused.
- `replace_path` can disable vanilla or other mod content unexpectedly.
