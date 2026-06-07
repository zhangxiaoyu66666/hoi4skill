# Technology Trees

Use this when the request asks for 科技树, 独有科技, technology folders, research categories, or technology icons.

## Local Research Notes

Vanilla HOI4 keeps researchable technology definitions in `common/technologies/*.txt` under one top-level `technologies = { ... }` wrapper. The local game install shows files such as `industry.txt`, `infantry.txt`, `air_techs.txt`, `armor.txt`, and `special_projects_tech.txt`.

KXC and MDARD currently show mostly technology art/GFX rather than full `common/technologies` edits. KXC has `interface/KRI_technologies.gfx`, with country-specific technology sprites such as `GFX_CPC_..._medium` and textures under `gfx/interface/technologies/...`. Do not infer a full custom research tree from icon files alone.

## Core Technology Shape

Typical visible technology entries include:

```hoi4
technologies = {
	my_mod_rail_dispatch_tech = {
		research_cost = 2
		start_year = 1938
		path = {
			leads_to_tech = my_mod_rail_network_tech
			research_cost_coeff = 1
		}
		folder = {
			name = industry_folder
			position = { x = 2 y = 6 }
		}
		categories = {
			industry
			construction_tech
		}
		ai_will_do = {
			factor = 1
		}
	}
}
```

Important fields:

- `folder = { name = <folder_id> position = { x = <col> y = <row> } }` places the technology in the visible research tree.
- `path = { leads_to_tech = <id> research_cost_coeff = <n> }` draws and prices a dependency path.
- `XOR = { <other_tech> }` creates mutually exclusive research choices.
- `categories = { ... }` feeds research bonuses, scripted references, and tech-category checks.
- `allow = { ... }` can hide or gate technologies.
- Equipment/building unlocks use specific blocks such as `enable_equipments`, `enable_equipment_modules`, or `enable_building`.
- `on_research_complete = { ... }` is for effects that fire when the technology finishes.

## Localisation And Mod Names

Technology localisation uses the technology ID and optional `_desc`:

```yaml
l_simp_chinese:
  my_mod_rail_dispatch_tech:0 "铁路调度算法"
  my_mod_rail_dispatch_tech_desc:0 "铁路部门开始用统一算法调配车辆、煤炭与沿线工人，使军列与民用运输不再互相拖垮。"
```

Folder localisation uses `<folder_id>` and `<folder_id>_desc`, such as vanilla `industry_folder` and `industry_folder_desc`.

Do not generate mod display-name localisation such as `chinaprc_1979_mod_name:0 "..."`. Mod names belong in `descriptor.mod` and the launcher-side `.mod` file, for example `kxlor.mod`.

## Safe Generation Rules

- Prefer adding a single custom technology to an existing folder over creating a new research folder.
- Before adding to an existing mod, scan `common/technologies`, `localisation/**`, and `interface/*.gfx` for nearby folder IDs, category names, icon naming, and coordinate spacing.
- Do not claim a technology is fully integrated into the research tree if the generated file only contains a minimal `technologies = { ... }` skeleton.
- If a new folder is requested, also plan folder localisation and check whether the target mod has any custom research GUI assumptions.
- If custom icons are needed, register sprites under `interface/*.gfx` and use semantic English sprite/texture names. Technology sprites often use names like `GFX_<TAG>_<tech>_medium`.
- Verify category names against the game/mod index before final code generation.
- Keep generated unique technology IDs ASCII-only and ending with `_tech`.
