# HOI4 File Map

Use this as a routing guide when turning a request into files.

## Root Files

- `descriptor.mod`: mod metadata loaded by the launcher and game.
- `<mod-id>.mod`: launcher-side descriptor usually stored next to the mod folder, useful for local installs.

## Localisation

- `localisation/simp_chinese/*.yml`: Simplified Chinese.
- `localisation/english/*.yml`: English.
- First line must be a language header such as `l_simp_chinese:`.
- Keys usually match scripted IDs, for example `my_focus` and `my_focus_desc`.

## National Focuses

- `common/national_focus/*.txt`
- Typical block: `focus_tree = { id = ... country = { factor = 0 modifier = { add = 10 tag = GER } } focus = { ... } }`
- Focuses usually need title and description localisation:
  - `<focus_id>:0 "Title"`
  - `<focus_id>_desc:0 "Description"`

## Ideas And Spirits

- `common/ideas/*.txt`
- Use for national spirits, advisors, designers, and laws.
- Add localisation for idea key and optional `_desc`.
- Add icons only when the mod already has an icon convention or the request requires it.

## Events

- `events/*.txt`
- Start with one or more top-level `add_namespace = <prefix>` declarations.
- Event IDs use a declared `<namespace>.<number>`; numbers `1..200000` are valid.
- News events use `news_event = { ... }`; country events use `country_event = { ... }`.
- Add localisation for title, description, and options.

## Decisions

- `common/decisions/*.txt`
- Use a category plus decision entries.
- Add localisation for category, decision, and descriptions.
- Use scripted triggers/effects when conditions or effects will be reused.

## Countries And Leaders

- `common/country_tags/*.txt`: tag to `countries/<CountryFile>.txt` mapping.
- `common/countries/*.txt`: country graphical setup and color.
- `history/countries/*.txt`: starting politics, capital, leaders, technologies, equipment, ideas, OOB, and character recruitment.
- `common/characters/*.txt`: modern character-style leaders, advisors, generals, field marshals, navy leaders, and scientists.
- `common/country_leader/*.txt`: country leader trait definitions, usually under `leader_traits = { ... }`.
- See `country-creation-leaders.md` before creating a country or leader.

## States And Victory Points

- `history/states/*.txt`
- Use for factories, resources, owners, cores, victory points, and buildings.
- `map/definition.csv`
- Use as the province-ID index. The first field is the province ID; the fifth field is normally the province type such as `land`, `sea`, or `lake`.
- `history/countries/*.txt` `capital = ...` uses a province ID, not a state ID.
- `history/states` `id = ...` is the state ID, while `provinces = { ... }` and `victory_points = { <province_id> <points> }` use province IDs.
- Prefer editing only targeted state files. Avoid wholesale vanilla copies when a small change is enough.
- See `history-states-provinces.md` before editing state files or using province IDs.

## Scripted Helpers

- `common/scripted_effects/*.txt`
- `common/scripted_triggers/*.txt`
- `common/scripted_localisation/*.txt`
- Use these when the same logic appears more than once or when a feature needs clear reusable hooks.

## Interface And Graphics

- `interface/*.gfx`: sprite definitions.
- `gfx/interface/...`: icon image files.
- Use existing icon naming and dimensions when possible.
- Do not invent image paths without adding the asset or reusing a known existing vanilla/mod sprite.
