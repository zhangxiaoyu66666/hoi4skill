# History States And Provinces

Use this when the request mentions 历史文件, 州, 省份, 首都, 核心, owner, controller, victory point, 工厂, 资源, 基建, 防空, or map edits.

## File Roles

- `history/countries/*.txt`: country start state, including `capital`, politics, technologies, ideas, OOB, and leader/character recruitment.
- `history/states/*.txt`: state ownership, cores, victory points, buildings, resources, province membership, local supplies, and dated state-history overrides.
- `map/definition.csv`: province definitions. The first field is the province ID. In normal HOI4 files the fifth field is the province type such as `land`, `sea`, or `lake`; some files have no header.
- `history/units/*.txt`: OOB/unit start files referenced from country history. Do not confuse unit history with state history.

## State File Shape

Typical shape:

```hoi4
state = {
  id = 64
  name = "STATE_64"
  manpower = 7265029
  state_category = megalopolis

  resources = {
    steel = 8
  }

  history = {
    owner = GER
    controller = GER
    add_core_of = GER
    victory_points = { 6521 50 }
    buildings = {
      infrastructure = 4
      arms_factory = 5
      6521 = {
        land_facility = 1
      }
    }
  }

  provinces = {
    375 478 6521
  }
}
```

The `id` is the state ID. The numbers in `provinces = { ... }` are province IDs. `victory_points = { <province_id> <points> }` also uses province IDs.

## Critical ID Rules

- `capital = ...` in `history/countries` uses a province ID, not a state ID.
- A state can contain many provinces. Do not use `STATE_64` or `id = 64` as the capital unless a province index proves that `64` is also a valid intended province.
- Buildings and resources in `history/states` are state-level data. Effects such as `add_building_construction` must run in state scope or be wrapped with a state selector.
- If the target mod has no local `history/states` or `map/definition.csv`, state/province facts are unknown locally. Use `hoi4skill build-game-index` on the game root and dependency roots, or ask the user for explicit state/province IDs.

## Submod Rule

For a submod, never infer inherited state or province IDs from a Chinese place name alone. Dependency names in `descriptor.mod` only prove a dependency exists; they do not prove which state files, map files, or province IDs are available. Build or read an index from the dependency/game root before using IDs.

## Safer Edit Strategy

Prefer state-scoped scripted effects when the request is a reward or temporary scripted change:

```hoi4
random_owned_controlled_state = {
  limit = { is_core_of = ROOT }
  add_building_construction = {
    type = arms_factory
    level = 2
    instant_build = yes
  }
}
```

Edit `history/states/*.txt` directly only when all of these are true:

- the target state file is verified,
- the state ID and province list are known,
- the change is meant to affect the start date or map setup,
- the edit will not copy a large vanilla file just to add one small effect.

## `mod-knowledge` Fields

`hoi4skill mod-knowledge` reports:

- `history_state_files`: local `history/states/*.txt` files.
- `history_states`: state summaries with file, id, `STATE_*` name, owner, controller, cores, province count/sample, victory-point province IDs, buildings, and resources.
- `province_definitions`: local `map/definition.csv` summary with province count, type counts, and sample IDs.

Treat missing fields as unknown. Do not fill gaps with invented province IDs.

## Rust CLI Gate

Before direct history edits, run:

```text
hoi4skill plan-history-edit "M:\path\mod" --text "edit history/states owner for state_id 64" --state-id 64 [--province-id 6521] [--capital 6521] [--game-root "C:\path\Hearts of Iron IV"] [--mod-path "M:\path\dependency.mod"] [--output history_plan.json]
```

The command emits `hoi4skill.history_edit_plan.v1` JSON:

- `evidence`: local `history/states`, local `map/definition.csv`, dependency state/province facts, and optional game-index counts.
- `checks`: whether the requested state ID, province ID, and capital province ID are actually known.
- `decision`: recommended strategy, whether direct history editing is allowed, safe generated targets, warnings, and skipped reasons.
- `prompt_rules`: hard rules the AI must preserve in the next generation step.

If `direct_history_edit_allowed` is `false`, do not write `history/states` or invent IDs. Report the `skipped` entries to the user and either request `--game-root` / `--mod-path`, ask for explicit IDs, or generate a state-scoped scripted effect instead.
